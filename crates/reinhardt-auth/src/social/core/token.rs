//! OAuth2 token types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OAuth2 access token and associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
	/// Access token value
	pub access_token: String,

	/// Token type (typically "Bearer")
	pub token_type: String,

	/// Expiration timestamp
	pub expires_at: DateTime<Utc>,

	/// Refresh token (if provided)
	pub refresh_token: Option<String>,

	/// Refresh token expiration timestamp (if the provider reports one)
	///
	/// Computed from [`TokenResponse::refresh_token_expires_in`]. `None` means
	/// the provider did not report a refresh token lifetime.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub refresh_token_expires_at: Option<DateTime<Utc>>,

	/// Granted scopes
	pub scopes: Vec<String>,

	/// ID token (OIDC only)
	pub id_token: Option<String>,
}

/// Token response from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
	/// Access token
	pub access_token: String,

	/// Token type
	pub token_type: String,

	/// Expires in seconds (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expires_in: Option<u64>,

	/// Refresh token (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub refresh_token: Option<String>,

	/// Refresh token lifetime in seconds (optional)
	///
	/// Reported by providers with expiring refresh tokens, such as GitHub Apps
	/// with expiring user authorization tokens.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub refresh_token_expires_in: Option<u64>,

	/// Scope (space-separated string, optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scope: Option<String>,

	/// ID token (OIDC only, optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id_token: Option<String>,
}

impl TokenResponse {
	/// Convert to OAuthToken with expiration calculation
	pub fn to_oauth_token(&self) -> OAuthToken {
		let now = Utc::now();
		let expires_at = if let Some(expires_in) = self.expires_in {
			now + chrono::Duration::seconds(expires_in as i64)
		} else {
			// Default to 1 hour if not specified
			now + chrono::Duration::hours(1)
		};

		// An out-of-range lifetime is treated as unreported rather than panicking
		// on provider-supplied input.
		let refresh_token_expires_at = self.refresh_token_expires_in.and_then(|secs| {
			let secs = i64::try_from(secs).ok()?;
			now.checked_add_signed(chrono::Duration::try_seconds(secs)?)
		});

		let scopes = self
			.scope
			.as_ref()
			.map(|s| s.split_whitespace().map(String::from).collect())
			.unwrap_or_default();

		OAuthToken {
			access_token: self.access_token.clone(),
			token_type: self.token_type.clone(),
			expires_at,
			refresh_token: self.refresh_token.clone(),
			refresh_token_expires_at,
			scopes,
			id_token: self.id_token.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[test]
	fn test_token_response_to_oauth_token() {
		let response = TokenResponse {
			access_token: "access_123".to_string(),
			token_type: "Bearer".to_string(),
			expires_in: Some(3600),
			refresh_token: Some("refresh_456".to_string()),
			refresh_token_expires_in: None,
			scope: Some("openid email profile".to_string()),
			id_token: Some("id_token_789".to_string()),
		};

		let token = response.to_oauth_token();

		assert_eq!(token.access_token, "access_123");
		assert_eq!(token.token_type, "Bearer");
		assert_eq!(token.refresh_token, Some("refresh_456".to_string()));
		assert_eq!(token.scopes, vec!["openid", "email", "profile"]);
		assert_eq!(token.id_token, Some("id_token_789".to_string()));
		assert!(token.expires_at > Utc::now());
	}

	#[test]
	fn test_token_response_default_expiration() {
		let response = TokenResponse {
			access_token: "access_123".to_string(),
			token_type: "Bearer".to_string(),
			expires_in: None,
			refresh_token: None,
			refresh_token_expires_in: None,
			scope: None,
			id_token: None,
		};

		let token = response.to_oauth_token();

		// Should default to 1 hour
		let expected_expiry = Utc::now() + chrono::Duration::hours(1);
		assert!(token.expires_at > Utc::now());
		assert!(token.expires_at <= expected_expiry + chrono::Duration::seconds(10));
		assert_eq!(token.refresh_token_expires_at, None);
	}

	#[rstest]
	fn test_refresh_token_expiry_computed_from_lifetime() {
		// Arrange
		// GitHub App user-to-server refresh tokens live for 6 months.
		let lifetime_secs: u64 = 15_897_600;
		let response = TokenResponse {
			access_token: "ghu_access".to_string(),
			token_type: "bearer".to_string(),
			expires_in: Some(28_800),
			refresh_token: Some("ghr_refresh".to_string()),
			refresh_token_expires_in: Some(lifetime_secs),
			scope: None,
			id_token: None,
		};
		let lifetime = chrono::Duration::seconds(lifetime_secs as i64);
		let before = Utc::now();

		// Act
		let token = response.to_oauth_token();

		// Assert
		let after = Utc::now();
		let expires_at = token
			.refresh_token_expires_at
			.expect("refresh token expiry should be computed");
		assert!(expires_at >= before + lifetime);
		assert!(expires_at <= after + lifetime);
	}

	#[rstest]
	fn test_refresh_token_expiry_out_of_range_lifetime_is_unreported() {
		// Arrange
		let response = TokenResponse {
			access_token: "access".to_string(),
			token_type: "Bearer".to_string(),
			expires_in: Some(3600),
			refresh_token: Some("refresh".to_string()),
			refresh_token_expires_in: Some(u64::MAX),
			scope: None,
			id_token: None,
		};

		// Act
		let token = response.to_oauth_token();

		// Assert
		assert_eq!(token.refresh_token_expires_at, None);
	}

	#[test]
	fn test_token_serde() {
		let token = OAuthToken {
			access_token: "test_access".to_string(),
			token_type: "Bearer".to_string(),
			expires_at: Utc::now(),
			refresh_token: Some("test_refresh".to_string()),
			refresh_token_expires_at: None,
			scopes: vec!["read".to_string(), "write".to_string()],
			id_token: None,
		};

		// Serialize
		let json = serde_json::to_string(&token).unwrap();
		assert!(json.contains("test_access"));

		// Deserialize
		let deserialized: OAuthToken = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.access_token, token.access_token);
		assert_eq!(deserialized.scopes, token.scopes);
	}
}
