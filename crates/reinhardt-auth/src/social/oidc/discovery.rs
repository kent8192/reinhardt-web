//! OpenID Connect Discovery document fetching
//!
//! Implements fetching and caching of .well-known/openid-configuration

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::social::core::{OAuth2Client, SocialAuthError};
use crate::social::url_validation::{sanitize_url, validate_endpoint_url};
use url::Url;

/// OpenID Connect Discovery document
///
/// Contains metadata about the OIDC provider's endpoints and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OIDCDiscovery {
	/// Issuer URL
	pub issuer: String,
	/// Authorization endpoint URL
	pub authorization_endpoint: String,
	/// Token endpoint URL
	pub token_endpoint: String,
	/// JWKS URI
	pub jwks_uri: String,
	/// UserInfo endpoint URL (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub userinfo_endpoint: Option<String>,
	/// Supported scopes
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scopes_supported: Option<Vec<String>>,
	/// Supported response types
	#[serde(skip_serializing_if = "Option::is_none")]
	pub response_types_supported: Option<Vec<String>>,
	/// Supported grant types
	#[serde(skip_serializing_if = "Option::is_none")]
	pub grant_types_supported: Option<Vec<String>>,
	/// Supported subject types
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subject_types_supported: Option<Vec<String>>,
	/// Supported ID token signing algorithms
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id_token_signing_alg_values_supported: Option<Vec<String>>,
	/// Supported claims
	#[serde(skip_serializing_if = "Option::is_none")]
	pub claims_supported: Option<Vec<String>>,
}

/// Cached discovery document with expiration
#[derive(Debug, Clone)]
struct CachedDiscovery {
	document: OIDCDiscovery,
	expires_at: DateTime<Utc>,
}

impl CachedDiscovery {
	fn new(document: OIDCDiscovery, ttl: Duration) -> Self {
		Self {
			document,
			expires_at: Utc::now() + ttl,
		}
	}

	fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}
}

/// Discovery document client with caching, keyed by issuer URL
pub struct DiscoveryClient {
	client: OAuth2Client,
	cache: RwLock<HashMap<String, CachedDiscovery>>,
	cache_ttl: Duration,
}

impl DiscoveryClient {
	/// Creates a new discovery client with default TTL (24 hours)
	pub fn new(client: OAuth2Client) -> Self {
		Self {
			client,
			cache: RwLock::new(HashMap::new()),
			cache_ttl: Duration::hours(24),
		}
	}

	/// Creates a new discovery client with custom TTL
	pub fn with_ttl(client: OAuth2Client, cache_ttl: Duration) -> Self {
		Self {
			client,
			cache: RwLock::new(HashMap::new()),
			cache_ttl,
		}
	}

	/// Fetches the discovery document from the issuer
	///
	/// # Arguments
	///
	/// * `issuer_url` - The OIDC issuer URL (e.g., `<https://accounts.google.com>`)
	///
	/// # Returns
	///
	/// The discovery document, either from cache or freshly fetched.
	pub async fn discover(&self, issuer_url: &str) -> Result<OIDCDiscovery, SocialAuthError> {
		self.discover_with_policy(issuer_url, false).await
	}

	/// Fetches a discovery document and requires all endpoints to stay on the issuer origin.
	///
	/// This stricter policy is intended for arbitrary IdP configurations where
	/// discovery metadata might be tenant-controlled. Built-in providers may use
	/// [`DiscoveryClient::discover`] because well-known providers can legitimately
	/// publish endpoints on dedicated OAuth or API hosts.
	pub async fn discover_same_origin(
		&self,
		issuer_url: &str,
	) -> Result<OIDCDiscovery, SocialAuthError> {
		self.discover_with_policy(issuer_url, true).await
	}

	async fn discover_with_policy(
		&self,
		issuer_url: &str,
		require_same_origin: bool,
	) -> Result<OIDCDiscovery, SocialAuthError> {
		// Check cache first
		{
			let cache = self.cache.read().await;
			if let Some(cached) = cache.get(issuer_url)
				&& !cached.is_expired()
			{
				return Ok(cached.document.clone());
			}
		}

		// Fetch from network
		let discovery_url = format!("{}/.well-known/openid-configuration", issuer_url);
		let response = self
			.client
			.client()
			.get(&discovery_url)
			.send()
			.await
			.map_err(|e| SocialAuthError::Network(e.to_string()))?;

		if !response.status().is_success() {
			return Err(SocialAuthError::Discovery(format!(
				"Discovery request failed: {}",
				response.status()
			)));
		}

		let document: OIDCDiscovery = response
			.json()
			.await
			.map_err(|e| SocialAuthError::Discovery(e.to_string()))?;

		// Validate all endpoint URLs once at discovery time. Arbitrary Generic
		// OIDC metadata uses the stricter same-origin policy to prevent a
		// discovery document from pivoting later server-side requests to an
		// unrelated host.
		validate_discovered_endpoint_url(
			issuer_url,
			&document.authorization_endpoint,
			require_same_origin,
		)?;
		validate_discovered_endpoint_url(
			issuer_url,
			&document.token_endpoint,
			require_same_origin,
		)?;
		validate_discovered_endpoint_url(issuer_url, &document.jwks_uri, require_same_origin)?;
		if let Some(ref userinfo_url) = document.userinfo_endpoint {
			validate_discovered_endpoint_url(issuer_url, userinfo_url, require_same_origin)?;
		}

		// Update cache keyed by issuer_url
		{
			let mut cache = self.cache.write().await;
			cache.insert(
				issuer_url.to_string(),
				CachedDiscovery::new(document.clone(), self.cache_ttl),
			);
		}

		Ok(document)
	}

	/// Clears the cache
	pub async fn clear_cache(&self) {
		let mut cache = self.cache.write().await;
		cache.clear();
	}
}

/// Validates that a discovery-provided endpoint stays on the issuer origin.
///
/// This prevents a malicious discovery document from turning later token,
/// JWKS, or UserInfo requests into server-side requests to unrelated hosts.
fn validate_discovered_endpoint_url(
	issuer_url: &str,
	endpoint_url: &str,
	require_same_origin: bool,
) -> Result<(), SocialAuthError> {
	validate_endpoint_url(endpoint_url)?;
	if !require_same_origin {
		return Ok(());
	}

	let issuer = Url::parse(issuer_url)
		.map_err(|e| SocialAuthError::Configuration(format!("invalid issuer URL: {}", e)))?;
	let endpoint = Url::parse(endpoint_url)
		.map_err(|e| SocialAuthError::Configuration(format!("invalid endpoint URL: {}", e)))?;

	if issuer.scheme() == endpoint.scheme()
		&& issuer.host_str() == endpoint.host_str()
		&& issuer.port_or_known_default() == endpoint.port_or_known_default()
	{
		return Ok(());
	}

	Err(SocialAuthError::InsecureEndpoint(format!(
		"discovered endpoint '{}' is outside issuer origin '{}'",
		sanitize_url(&endpoint),
		sanitize_url(&issuer)
	)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cached_discovery_expiration() {
		let document = OIDCDiscovery {
			issuer: "https://example.com".to_string(),
			authorization_endpoint: "https://example.com/auth".to_string(),
			token_endpoint: "https://example.com/token".to_string(),
			jwks_uri: "https://example.com/jwks".to_string(),
			userinfo_endpoint: None,
			scopes_supported: None,
			response_types_supported: None,
			grant_types_supported: None,
			subject_types_supported: None,
			id_token_signing_alg_values_supported: None,
			claims_supported: None,
		};

		let cached = CachedDiscovery::new(document, Duration::seconds(1));
		assert!(!cached.is_expired());

		let expired = CachedDiscovery::new(cached.document.clone(), Duration::seconds(-1));
		assert!(expired.is_expired());
	}

	#[test]
	fn test_discovered_endpoint_allows_same_origin() {
		// Arrange
		let issuer = "https://issuer.example.com/auth";
		let endpoint = "https://issuer.example.com/token";

		// Act
		let result = validate_discovered_endpoint_url(issuer, endpoint, true);

		// Assert
		assert!(result.is_ok());
	}

	#[test]
	fn test_discovered_endpoint_rejects_different_host() {
		// Arrange
		let issuer = "https://issuer.example.com";
		let endpoint = "https://127.0.0.1/token";

		// Act
		let result = validate_discovered_endpoint_url(issuer, endpoint, true);

		// Assert
		let err = result.expect_err("different host must be rejected");
		assert!(matches!(err, SocialAuthError::InsecureEndpoint(_)));
	}

	#[test]
	fn test_discovered_endpoint_rejects_different_port() {
		// Arrange
		let issuer = "https://issuer.example.com";
		let endpoint = "https://issuer.example.com:8443/token";

		// Act
		let result = validate_discovered_endpoint_url(issuer, endpoint, true);

		// Assert
		let err = result.expect_err("different port must be rejected");
		assert!(matches!(err, SocialAuthError::InsecureEndpoint(_)));
	}

	#[test]
	fn test_discovered_endpoint_allows_same_loopback_http_origin_for_dev() {
		// Arrange
		let issuer = "http://127.0.0.1:8080";
		let endpoint = "http://127.0.0.1:8080/token";

		// Act
		let result = validate_discovered_endpoint_url(issuer, endpoint, true);

		// Assert
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_client_creation() {
		let client = OAuth2Client::new();
		let discovery_client = DiscoveryClient::new(client);
		assert!(discovery_client.cache.read().await.is_empty());
	}

	#[tokio::test]
	async fn test_client_with_custom_ttl() {
		let client = OAuth2Client::new();
		let discovery_client = DiscoveryClient::with_ttl(client, Duration::hours(1));
		assert_eq!(discovery_client.cache_ttl, Duration::hours(1));
	}

	#[tokio::test]
	async fn test_clear_cache() {
		let client = OAuth2Client::new();
		let discovery_client = DiscoveryClient::new(client);

		// Manually set cache with a key
		{
			let document = OIDCDiscovery {
				issuer: "https://example.com".to_string(),
				authorization_endpoint: "https://example.com/auth".to_string(),
				token_endpoint: "https://example.com/token".to_string(),
				jwks_uri: "https://example.com/jwks".to_string(),
				userinfo_endpoint: None,
				scopes_supported: None,
				response_types_supported: None,
				grant_types_supported: None,
				subject_types_supported: None,
				id_token_signing_alg_values_supported: None,
				claims_supported: None,
			};
			let mut cache = discovery_client.cache.write().await;
			cache.insert(
				"https://example.com".to_string(),
				CachedDiscovery::new(document, Duration::hours(1)),
			);
		}

		assert!(!discovery_client.cache.read().await.is_empty());

		discovery_client.clear_cache().await;
		assert!(discovery_client.cache.read().await.is_empty());
	}
}
