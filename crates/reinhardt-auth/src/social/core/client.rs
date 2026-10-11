//! HTTP client wrapper for OAuth2 requests

use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use url::Url;

use super::error::SocialAuthError;
use crate::social::url_validation::{is_public_ip, validate_endpoint_url};

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
			.timeout(Duration::from_secs(30))
			.connect_timeout(Duration::from_secs(10))
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
	/// Redirects and environment proxies cannot move the request after validation.
	pub(crate) async fn for_endpoint(&self, endpoint: &str) -> Result<Client, SocialAuthError> {
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
			url::Host::Domain(domain) => tokio::net::lookup_host((*domain, port))
				.await
				.map_err(|e| SocialAuthError::Network(e.to_string()))?
				.collect(),
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
		let mut builder = Client::builder()
			.timeout(Duration::from_secs(30))
			.connect_timeout(Duration::from_secs(10))
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
