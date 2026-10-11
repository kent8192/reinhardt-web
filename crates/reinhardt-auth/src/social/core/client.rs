//! HTTP client wrapper for OAuth2 requests

use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use url::Url;

use super::error::SocialAuthError;
use crate::social::url_validation::{is_public_ip, validate_endpoint_url};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

async fn resolve_endpoint_addresses<R, F>(
	domain: &str,
	port: u16,
	deadline: Duration,
	resolver: R,
) -> Result<Vec<std::net::SocketAddr>, SocialAuthError>
where
	R: FnOnce(String, u16) -> F,
	F: std::future::Future<Output = std::io::Result<Vec<std::net::SocketAddr>>>,
{
	tokio::time::timeout(deadline, resolver(domain.to_owned(), port))
		.await
		.map_err(|_| SocialAuthError::Network("endpoint DNS resolution timed out".to_string()))?
		.map_err(|error| SocialAuthError::Network(error.to_string()))
}

fn allowed_resolved_ip(ip: std::net::IpAddr, scheme: &str, allow_loopback: bool) -> bool {
	if scheme == "http" {
		return allow_loopback && ip.is_loopback();
	}
	is_public_ip(ip) || (allow_loopback && ip.is_loopback())
}

/// OAuth2 HTTP client
///
/// Wrapper around `reqwest::Client` with OAuth2-specific configuration.
/// Cloning is cheap since `reqwest::Client` uses an `Arc` internally.
#[derive(Clone)]
pub struct OAuth2Client {
	client: Client,
}

impl OAuth2Client {
	/// Create a new OAuth2 client
	pub fn new() -> Self {
		let client = Client::builder()
			.timeout(REQUEST_TIMEOUT)
			.connect_timeout(CONNECT_TIMEOUT)
			.redirect(Policy::none())
			.build()
			.expect("Failed to build HTTP client");

		Self { client }
	}

	/// Create an OAuth2 client that returns redirect responses without following them.
	///
	/// This is used for generic OIDC providers after discovery constrains endpoints
	/// to the issuer origin. Following a redirect could otherwise bypass that
	/// constraint after the initial endpoint URL has been validated.
	pub(crate) fn without_redirects() -> Self {
		Self::new()
	}

	/// Resolve and pin a validated endpoint for a single OAuth/OIDC request.
	/// DNS resolution has the ten-second connection deadline and consumes part of
	/// the thirty-second request budget. Redirects and environment proxies cannot
	/// move the request after validation.
	pub(crate) async fn for_endpoint(&self, endpoint: &str) -> Result<Client, SocialAuthError> {
		let started = std::time::Instant::now();
		validate_endpoint_url(endpoint)?;
		let parsed = Url::parse(endpoint)
			.map_err(|e| SocialAuthError::Configuration(format!("invalid endpoint URL: {e}")))?;
		let host = parsed.host().ok_or_else(|| {
			SocialAuthError::Configuration("endpoint URL has no host".to_string())
		})?;
		let port = parsed.port_or_known_default().ok_or_else(|| {
			SocialAuthError::Configuration("endpoint URL has no port".to_string())
		})?;
		let allow_loopback = crate::social::url_validation::is_loopback(&parsed);
		let addresses: Vec<_> = match &host {
			url::Host::Domain(domain) => {
				resolve_endpoint_addresses(
					domain,
					port,
					CONNECT_TIMEOUT,
					|domain, port| async move {
						tokio::net::lookup_host((domain, port))
							.await
							.map(Iterator::collect)
					},
				)
				.await?
			}
			url::Host::Ipv4(ip) => vec![std::net::SocketAddr::new((*ip).into(), port)],
			url::Host::Ipv6(ip) => vec![std::net::SocketAddr::new((*ip).into(), port)],
		};
		if addresses.is_empty()
			|| addresses
				.iter()
				.any(|addr| !allowed_resolved_ip(addr.ip(), parsed.scheme(), allow_loopback))
		{
			return Err(SocialAuthError::InsecureEndpoint(
				"endpoint resolves to a disallowed address".to_string(),
			));
		}
		let remaining = REQUEST_TIMEOUT.saturating_sub(started.elapsed());
		if remaining.is_zero() {
			return Err(SocialAuthError::Network(
				"endpoint request deadline exceeded".to_string(),
			));
		}
		let mut builder = Client::builder()
			.timeout(remaining)
			.connect_timeout(CONNECT_TIMEOUT)
			.redirect(Policy::none())
			.no_proxy();
		if let url::Host::Domain(domain) = host {
			builder = builder.resolve_to_addrs(domain, &addresses);
		}
		builder
			.build()
			.map_err(|e| SocialAuthError::Network(e.to_string()))
	}

	/// Get the underlying reqwest client
	pub fn client(&self) -> &Client {
		&self.client
	}
}

impl Default for OAuth2Client {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[rstest::rstest]
	#[case("192.0.0.8", false)]
	#[case("192.0.0.9", true)]
	#[case("192.0.0.10", true)]
	#[case("192.0.0.11", false)]
	#[case("192.0.0.170", false)]
	#[case("192.0.0.171", false)]
	#[tokio::test]
	async fn oauth_endpoints_apply_narrow_anycast_exceptions_to_literals_and_dns(
		#[case] address: &str,
		#[case] allowed: bool,
	) {
		// Arrange: the same destination is supplied directly and by an asynchronous resolver.
		let client = OAuth2Client::new();
		let socket: std::net::SocketAddr = format!("{address}:443").parse().unwrap();

		// Act: build a pinned literal client and use the production DNS/allowlist helpers.
		let direct = client
			.for_endpoint(&format!("https://{address}/token"))
			.await;
		let resolved = resolve_endpoint_addresses(
			"anycast.example",
			443,
			CONNECT_TIMEOUT,
			|domain, port| async move {
				assert_eq!(domain, "anycast.example");
				assert_eq!(port, 443);
				Ok(vec![socket])
			},
		)
		.await
		.unwrap();

		// Assert: credentials can target only the two /32 exceptions, never adjacent ranges.
		assert_eq!(direct.is_ok(), allowed);
		assert_eq!(
			resolved
				.iter()
				.all(|addr| allowed_resolved_ip(addr.ip(), "https", false)),
			allowed
		);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn pending_endpoint_dns_wait_expires_and_is_dropped() {
		use std::sync::{
			Arc,
			atomic::{AtomicBool, Ordering},
		};
		// Arrange: the production resolver wrapper awaits a delayed DNS future.
		let dropped = Arc::new(AtomicBool::new(false));
		struct LookupGuard(Arc<AtomicBool>);
		impl Drop for LookupGuard {
			fn drop(&mut self) {
				self.0.store(true, Ordering::Release);
			}
		}
		let flag = dropped.clone();
		let started = std::time::Instant::now();

		// Act: resolution exceeds a short test deadline.
		let error = resolve_endpoint_addresses(
			"slow.example",
			443,
			Duration::from_millis(30),
			|domain, port| async move {
				assert_eq!(domain, "slow.example");
				assert_eq!(port, 443);
				let _guard = LookupGuard(flag);
				tokio::time::sleep(Duration::from_secs(1)).await;
				Ok(vec!["8.8.8.8:443".parse().unwrap()])
			},
		)
		.await
		.unwrap_err();

		// Assert: the request receives a network error and the pending wait is dropped.
		assert!(
			matches!(error, SocialAuthError::Network(message) if message.contains("DNS resolution timed out"))
		);
		assert!(started.elapsed() < Duration::from_millis(500));
		assert!(dropped.load(Ordering::Acquire));
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn endpoint_dns_wrapper_preserves_success_and_errors() {
		// Arrange / Act: successful DNS answers and resolver errors use the same wrapper.
		let addresses =
			resolve_endpoint_addresses("public.example", 443, CONNECT_TIMEOUT, |_, _| async {
				Ok(vec!["8.8.8.8:443".parse().unwrap()])
			})
			.await
			.unwrap();
		let error =
			resolve_endpoint_addresses("missing.example", 443, CONNECT_TIMEOUT, |_, _| async {
				Err(std::io::Error::other("lookup unavailable"))
			})
			.await
			.unwrap_err();

		// Assert: neither path changes address pinning inputs or loses error context.
		assert_eq!(
			addresses,
			vec!["8.8.8.8:443".parse::<std::net::SocketAddr>().unwrap()]
		);
		assert!(
			matches!(error, SocialAuthError::Network(message) if message == "lookup unavailable")
		);
	}

	#[test]
	fn test_client_creation() {
		let client = OAuth2Client::new();
		// Verify that client() returns a valid reference
		let _client_ref = client.client();
	}

	#[test]
	fn test_client_default() {
		let client = OAuth2Client::default();
		// Verify that client() returns a valid reference
		let _client_ref = client.client();
	}

	#[tokio::test]
	async fn test_client_rejects_internal_endpoint_before_request() {
		let client = OAuth2Client::new();
		assert!(client.for_endpoint("https://10.0.0.1/token").await.is_err());
		assert!(
			client
				.for_endpoint("https://user@8.8.8.8/token")
				.await
				.is_err()
		);
		assert!(client.for_endpoint("https://8.8.8.8/token").await.is_ok());
		assert!(client.for_endpoint("http://[::1]/token").await.is_ok());
	}

	#[rstest::rstest]
	fn http_loopback_endpoint_cannot_resolve_to_public_address() {
		let public_ip = "8.8.8.8".parse().unwrap();
		let loopback_ip = "127.0.0.1".parse().unwrap();
		assert!(!allowed_resolved_ip(public_ip, "http", true));
		assert!(allowed_resolved_ip(loopback_ip, "http", true));
		assert!(allowed_resolved_ip(public_ip, "https", false));
	}
}
