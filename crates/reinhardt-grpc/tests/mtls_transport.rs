#![cfg(feature = "tls")]

use rcgen::{
	BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
	KeyUsagePurpose, SanType,
};
use reinhardt_grpc::proto::graphql::{
	GraphQlRequest, GraphQlResponse, SubscriptionEvent,
	graph_ql_service_client::GraphQlServiceClient,
	graph_ql_service_server::{GraphQlService, GraphQlServiceServer},
};
use reinhardt_grpc::{
	GrpcClientTlsConfig, GrpcServerTlsConfig, PeerIdentity, PeerIdentityInterceptor,
};
use rstest::{fixture, rstest};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status};

const ALLOWED_URI: &str = "spiffe://example.test/service/marketplace";

struct Pki {
	ca: String,
	issuer: Issuer<'static, KeyPair>,
}

#[fixture]
fn pki() -> Pki {
	let mut params = CertificateParams::default();
	params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
	params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
	let key = KeyPair::generate().unwrap();
	let ca = params.self_signed(&key).unwrap().pem();
	Pki {
		ca,
		issuer: Issuer::new(params, key),
	}
}

impl Pki {
	fn leaf(&self, uri: Option<&str>, server: bool) -> (String, String) {
		let mut params = CertificateParams::new(vec!["auth.internal".into()]).unwrap();
		if let Some(uri) = uri {
			params
				.subject_alt_names
				.push(SanType::URI(uri.try_into().unwrap()));
		}
		params.extended_key_usages = vec![if server {
			ExtendedKeyUsagePurpose::ServerAuth
		} else {
			ExtendedKeyUsagePurpose::ClientAuth
		}];
		let key = KeyPair::generate().unwrap();
		(
			params.signed_by(&key, &self.issuer).unwrap().pem(),
			key.serialize_pem(),
		)
	}
}

struct Echo;

#[tonic::async_trait]
impl GraphQlService for Echo {
	async fn execute_query(
		&self,
		request: Request<GraphQlRequest>,
	) -> Result<Response<GraphQlResponse>, Status> {
		let peer = request
			.extensions()
			.get::<PeerIdentity>()
			.ok_or_else(|| Status::internal("identity missing"))?;
		Ok(Response::new(GraphQlResponse {
			data: Some(peer.uri_sans().join(",")),
			errors: vec![],
			extensions: None,
		}))
	}

	async fn execute_mutation(
		&self,
		request: Request<GraphQlRequest>,
	) -> Result<Response<GraphQlResponse>, Status> {
		self.execute_query(request).await
	}

	type ExecuteSubscriptionStream = tokio_stream::Empty<Result<SubscriptionEvent, Status>>;

	async fn execute_subscription(
		&self,
		_: Request<GraphQlRequest>,
	) -> Result<Response<Self::ExecuteSubscriptionStream>, Status> {
		Err(Status::unimplemented("subscriptions are outside this test"))
	}
}

struct RunningServer {
	endpoint: String,
	task: JoinHandle<Result<(), tonic::transport::Error>>,
}

impl Drop for RunningServer {
	fn drop(&mut self) {
		self.task.abort();
	}
}

async fn server(pki: &Pki) -> RunningServer {
	let (cert, key) = pki.leaf(None, true);
	let mut transport = GrpcServerTlsConfig::new(cert, key, pki.ca.clone())
		.configure(tonic::transport::Server::builder())
		.unwrap();
	let authorize = PeerIdentityInterceptor::new(|peer: &PeerIdentity| {
		if peer.uri_sans() == [ALLOWED_URI] {
			Ok(())
		} else {
			Err(Status::permission_denied("service is not authorized"))
		}
	});
	let service = GraphQlServiceServer::with_interceptor(Echo, authorize);
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("https://{}", listener.local_addr().unwrap());
	let task = tokio::spawn(
		transport
			.add_service(service)
			.serve_with_incoming(TcpListenerStream::new(listener)),
	);
	RunningServer { endpoint, task }
}

#[derive(Clone, Copy)]
enum ClientCase {
	Allowed,
	WrongUri,
	MissingUri,
	WrongCa,
	MissingCertificate,
}

// Guards the real transport and authorization boundary: removing CA verification,
// accepting metadata as identity, or skipping service policy changes these results.
#[rstest]
#[case::allowed(ClientCase::Allowed, None)]
#[case::wrong_service(ClientCase::WrongUri, Some(tonic::Code::PermissionDenied))]
#[case::no_uri_identity(ClientCase::MissingUri, Some(tonic::Code::Unauthenticated))]
#[case::untrusted_client(ClientCase::WrongCa, Some(tonic::Code::Unavailable))]
#[case::no_client_certificate(ClientCase::MissingCertificate, Some(tonic::Code::Unavailable))]
#[tokio::test]
async fn mtls_authenticates_peer_and_enforces_service_policy(
	pki: Pki,
	#[case] case: ClientCase,
	#[case] expected: Option<tonic::Code>,
) {
	// Arrange
	let server = server(&pki).await;
	let other_pki = self::pki();
	let signer = if matches!(case, ClientCase::WrongCa) {
		&other_pki
	} else {
		&pki
	};
	let uri = match case {
		ClientCase::MissingUri => None,
		ClientCase::WrongUri => Some("spiffe://example.test/service/unrelated"),
		_ => Some(ALLOWED_URI),
	};
	let (cert, key) = signer.leaf(uri, false);

	// Act
	let result = tokio::time::timeout(Duration::from_secs(5), async {
		let channel = if matches!(case, ClientCase::MissingCertificate) {
			tonic::transport::Endpoint::from_shared(server.endpoint.clone())
				.unwrap()
				.tls_config(
					tonic::transport::ClientTlsConfig::new()
						.ca_certificate(tonic::transport::Certificate::from_pem(pki.ca.clone()))
						.domain_name("auth.internal"),
				)
				.unwrap()
				.connect()
				.await
				.map_err(|_| Status::unavailable("TLS connection rejected"))
		} else {
			GrpcClientTlsConfig::new(pki.ca.clone(), cert, key, "auth.internal")
				.connect(&server.endpoint)
				.await
				.map_err(|_| Status::unavailable("TLS connection rejected"))
		};
		let channel = channel.map_err(|_| Status::unavailable("TLS connection rejected"))?;
		let mut client = GraphQlServiceClient::new(channel);
		let mut request = Request::new(GraphQlRequest {
			query: "identity".into(),
			variables: None,
			operation_name: None,
		});
		// A caller-controlled identity header must not affect authorization.
		request
			.metadata_mut()
			.insert("x-service-identity", ALLOWED_URI.parse().unwrap());
		client.execute_query(request).await
	})
	.await
	.expect("bounded TLS request");

	// Assert
	match expected {
		None => assert_eq!(
			result.unwrap().into_inner().data.as_deref(),
			Some(ALLOWED_URI)
		),
		Some(tonic::Code::Unavailable) => {
			// A TLS alert has no gRPC status; Tonic can report Unknown when
			// the peer closes before sending HTTP/2 headers.
			assert!(matches!(
				result.unwrap_err().code(),
				tonic::Code::Unavailable | tonic::Code::Unknown
			));
		}
		Some(code) => assert_eq!(result.unwrap_err().code(), code),
	}
}

#[rstest]
#[case::wrong_server_name("wrong.internal", false)]
#[case::wrong_server_ca("auth.internal", true)]
#[tokio::test]
async fn client_rejects_untrusted_server(pki: Pki, #[case] domain: &str, #[case] wrong_ca: bool) {
	// Arrange
	let server = server(&pki).await;
	let (cert, key) = pki.leaf(Some(ALLOWED_URI), false);
	let ca = if wrong_ca { self::pki().ca } else { pki.ca };

	// Act
	let result = GrpcClientTlsConfig::new(ca, cert, key, domain)
		.connect(&server.endpoint)
		.await;

	// Assert
	assert!(
		result.is_err(),
		"server trust and hostname must both be verified"
	);
}

#[rstest]
#[tokio::test]
async fn client_refuses_plaintext_endpoint(pki: Pki) {
	// Arrange
	let (cert, key) = pki.leaf(Some(ALLOWED_URI), false);

	// Act
	let result = GrpcClientTlsConfig::new(pki.ca, cert, key, "auth.internal")
		.connect("http://127.0.0.1:1")
		.await;

	// Assert
	assert!(
		result.is_err(),
		"mTLS client must never fall back to plaintext"
	);
}

#[rstest]
fn debug_does_not_disclose_client_private_key() {
	// Arrange
	let key = b"sensitive-private-key-material";
	let config = GrpcClientTlsConfig::new(b"ca", b"cert", key, "auth.internal");

	// Act
	let diagnostic = format!("{config:?}");

	// Assert: Tonic stores the key as bytes, so check both likely representations.
	assert!(!diagnostic.contains(std::str::from_utf8(key).unwrap()));
	assert!(!diagnostic.contains(&format!("{key:?}")));
}
