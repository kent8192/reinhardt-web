//! Shared token endpoint response handling
//!
//! Parses responses from OAuth2 token endpoints for the authorization-code
//! exchange and refresh flows.

use serde::Deserialize;

use crate::social::core::{SocialAuthError, TokenResponse};

/// OAuth2 error body (RFC 6749 Section 5.2).
///
/// Some providers (notably GitHub) return this body with HTTP 200 instead of
/// a 4xx status, so it must be checked before parsing a token response.
#[derive(Deserialize)]
struct OAuthErrorBody {
	error: String,
	#[serde(default)]
	error_description: Option<String>,
}

/// Converts a token endpoint response into a [`TokenResponse`].
///
/// `operation` names the request in error messages (for example
/// `"Token refresh"`), and `to_error` selects the error variant reported for
/// every failure except body read errors, which are reported as
/// [`SocialAuthError::Network`].
pub(crate) async fn parse_token_endpoint_response(
	response: reqwest::Response,
	operation: &str,
	to_error: fn(String) -> SocialAuthError,
) -> Result<TokenResponse, SocialAuthError> {
	let status = response.status();

	if !status.is_success() {
		let error_body = response
			.text()
			.await
			.unwrap_or_else(|_| "Unknown error".to_string());

		return Err(to_error(format!(
			"{} failed ({}): {}",
			operation, status, error_body
		)));
	}

	let body = response
		.text()
		.await
		.map_err(|e| SocialAuthError::Network(e.to_string()))?;

	if let Ok(error_body) = serde_json::from_str::<OAuthErrorBody>(&body) {
		let message = match error_body.error_description {
			Some(description) => format!(
				"{} failed: {}: {}",
				operation, error_body.error, description
			),
			None => format!("{} failed: {}", operation, error_body.error),
		};
		return Err(to_error(message));
	}

	serde_json::from_str(&body).map_err(|e| to_error(e.to_string()))
}
