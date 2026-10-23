//! Mutual TLS transport and certificate-based service identity.
//!
//! Enable `tls` on this crate, or `grpc-tls` on the Reinhardt facade. Trust roots
//! are explicit: neither client nor server uses platform roots or plaintext
//! fallback. Certificate issuance and rotation belong to the application.
//!
//! ```rust,no_run
//! use reinhardt_grpc::{GrpcClientTlsConfig, GrpcServerTlsConfig};
//! use reinhardt_grpc::tonic::transport::Server;
//!
//! # async fn connect() -> Result<(), Box<dyn std::error::Error>> {
//! let server_cert = std::fs::read("server.pem")?;
//! let server_key = std::fs::read("server-key.pem")?;
//! let ca = std::fs::read("ca.pem")?;
//! let _server = GrpcServerTlsConfig::new(server_cert, server_key, &ca)
//!     .configure(Server::builder())?;
//! let client_cert = std::fs::read("client.pem")?;
//! let client_key = std::fs::read("client-key.pem")?;
//! let _channel = GrpcClientTlsConfig::new(ca, client_cert, client_key, "auth.internal")
//!     .endpoint("https://127.0.0.1:50051")?
//!     .connect_timeout(std::time::Duration::from_secs(5))
//!     .connect().await?;
//! # Ok(())
//! # }
//! ```

use tonic::transport::server::{TcpConnectInfo, TlsConnectInfo};
use tonic::transport::{
	Certificate, Channel, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig,
};
use tonic::{Request, Status};
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::{FromDer, X509Certificate};

/// Errors configuring or connecting an authenticated gRPC client.
#[derive(Debug, thiserror::Error)]
pub enum GrpcTlsError {
	/// The endpoint must use HTTPS; plaintext fallback is forbidden.
	#[error("mutual TLS requires an HTTPS endpoint")]
	HttpsRequired,
	/// TLS configuration, endpoint parsing, or transport failed.
	#[error(transparent)]
	Transport(#[from] tonic::transport::Error),
}

/// Server identity and mandatory client certificate trust roots.
///
/// Every accepted connection must present a client certificate chaining to the
/// supplied CA bundle. Attach `PeerIdentityInterceptor` to each generated service
/// to additionally authorize the service identity; trusting a CA alone does not
/// authorize every certificate holder for every RPC service.
#[derive(Clone, Debug)]
pub struct GrpcServerTlsConfig {
	tls: ServerTlsConfig,
}

impl GrpcServerTlsConfig {
	/// Creates an mTLS configuration from PEM certificate chain, key, and client CA bundle.
	///
	/// Certificate material is validated by [`Self::configure`], before serving.
	pub fn new(cert: impl AsRef<[u8]>, key: impl AsRef<[u8]>, client_ca: impl AsRef<[u8]>) -> Self {
		Self {
			tls: ServerTlsConfig::new()
				.identity(Identity::from_pem(cert, key))
				.client_ca_root(Certificate::from_pem(client_ca))
				.client_auth_optional(false),
		}
	}

	/// Applies mandatory mTLS to a Tonic server builder.
	pub fn configure(&self, server: Server) -> Result<Server, tonic::transport::Error> {
		server.tls_config(self.tls.clone())
	}
}

/// Client identity, explicit server CA bundle, and expected server DNS identity.
///
/// The configured DNS name is checked against the server certificate even when
/// connecting to an IP address. The client certificate is sent only during TLS.
#[derive(Clone, Debug)]
pub struct GrpcClientTlsConfig {
	tls: ClientTlsConfig,
}

impl GrpcClientTlsConfig {
	/// Creates an authenticated client configuration from PEM material.
	///
	/// The server must chain to `server_ca` and match `server_domain`.
	pub fn new(
		server_ca: impl AsRef<[u8]>,
		cert: impl AsRef<[u8]>,
		key: impl AsRef<[u8]>,
		server_domain: impl Into<String>,
	) -> Self {
		Self {
			tls: ClientTlsConfig::new()
				.ca_certificate(Certificate::from_pem(server_ca))
				.identity(Identity::from_pem(cert, key))
				.domain_name(server_domain),
		}
	}

	/// Creates an HTTPS-only endpoint with mTLS configured.
	///
	/// Callers can apply Tonic timeout, concurrency, and keepalive settings to
	/// the returned endpoint before connecting. No connection is opened here.
	pub fn endpoint(&self, endpoint: &str) -> Result<Endpoint, GrpcTlsError> {
		let endpoint = Endpoint::from_shared(endpoint.to_owned())?;
		if endpoint.uri().scheme_str() != Some("https") {
			return Err(GrpcTlsError::HttpsRequired);
		}
		Ok(endpoint.tls_config(self.tls.clone())?)
	}

	/// Connects an authenticated channel without plaintext fallback.
	pub async fn connect(&self, endpoint: &str) -> Result<Channel, GrpcTlsError> {
		Ok(self.endpoint(endpoint)?.connect().await?)
	}
}

/// URI SAN identities from the leaf certificate of a verified TLS connection.
///
/// Constructed exclusively from transport connection information, never request
/// metadata or the certificate common name. URI semantics and trust-domain
/// allowlists are application policy. Compare complete URI strings rather than
/// matching substrings. The transport must use mandatory certificate verification
/// (as configured by `GrpcServerTlsConfig`); this extractor does not re-verify
/// certificate chains or authenticate user-supplied request extensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerIdentity {
	uri_sans: Vec<String>,
}

impl PeerIdentity {
	/// Extracts URI SANs from the authenticated peer's leaf certificate.
	///
	/// Returns `Unauthenticated` for plaintext, absent certificates, malformed
	/// certificates, duplicate SAN extensions, or a leaf with no URI identity.
	/// Supports Tonic's native TCP/TLS transport, including `TcpListenerStream`.
	pub fn from_request<T>(request: &Request<T>) -> Result<Self, Status> {
		let invalid = || Status::unauthenticated("authenticated service identity required");
		let connection = request
			.extensions()
			.get::<TlsConnectInfo<TcpConnectInfo>>()
			.ok_or_else(invalid)?;
		let certificates = connection.peer_certs().ok_or_else(invalid)?;
		let leaf = certificates.first().ok_or_else(invalid)?;
		let (remaining, cert) = X509Certificate::from_der(leaf.as_ref()).map_err(|_| invalid())?;
		if !remaining.is_empty() {
			return Err(invalid());
		}
		let san = cert
			.subject_alternative_name()
			.map_err(|_| invalid())?
			.ok_or_else(invalid)?;
		let uri_sans: Vec<_> = san
			.value
			.general_names
			.iter()
			.filter_map(|name| match name {
				GeneralName::URI(uri) if !uri.is_empty() => Some((*uri).to_owned()),
				_ => None,
			})
			.collect();
		if uri_sans.is_empty() {
			return Err(invalid());
		}
		Ok(Self { uri_sans })
	}

	/// Returns all URI SANs from the authenticated leaf certificate.
	pub fn uri_sans(&self) -> &[String] {
		&self.uri_sans
	}
}

/// Authenticates certificate identity and applies a per-service authorization policy.
///
/// Pass to a generated service's `with_interceptor` constructor. The callback
/// must explicitly allow the authenticated peer; rejected calls never reach the
/// handler. On success, handlers can read `PeerIdentity` from request extensions.
/// A separate interceptor per generated service allows separate service policies.
/// The callback is synchronous; async or per-method business authorization belongs
/// in the handler or an application middleware layer.
///
/// ```rust
/// use reinhardt_grpc::{PeerIdentity, PeerIdentityInterceptor};
/// use reinhardt_grpc::tonic::Status;
/// let _policy = PeerIdentityInterceptor::new(|peer: &PeerIdentity| {
///     if peer.uri_sans().iter().any(|uri| uri == "spiffe://example.test/marketplace") {
///         Ok(())
///     } else {
///         Err(Status::permission_denied("service is not authorized"))
///     }
/// });
/// ```
#[derive(Clone)]
pub struct PeerIdentityInterceptor<F> {
	authorize: F,
}

impl<F> PeerIdentityInterceptor<F> {
	/// Creates a certificate identity interceptor with an explicit allow policy.
	pub fn new(authorize: F) -> Self {
		Self { authorize }
	}
}

impl<F> tonic::service::Interceptor for PeerIdentityInterceptor<F>
where
	F: FnMut(&PeerIdentity) -> Result<(), Status>,
{
	fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
		let peer = PeerIdentity::from_request(&request)?;
		(self.authorize)(&peer)?;
		request.extensions_mut().insert(peer);
		Ok(request)
	}
}
