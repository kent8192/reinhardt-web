//! AWS credential helpers.

use std::env;

use crate::{ProviderError, Result};

/// AWS credentials used for signing provider requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsCredentials {
	access_key_id: String,
	secret_access_key: String,
	session_token: Option<String>,
}

impl AwsCredentials {
	/// Create AWS credentials without a session token.
	pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
		Self {
			access_key_id: access_key_id.into(),
			secret_access_key: secret_access_key.into(),
			session_token: None,
		}
	}

	/// Attach a session token to these credentials.
	#[must_use]
	pub fn with_session_token(mut self, session_token: impl Into<String>) -> Self {
		self.session_token = Some(session_token.into());
		self
	}

	/// Load AWS credentials from environment variables.
	///
	/// Reads `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optional
	/// `AWS_SESSION_TOKEN`.
	///
	/// # Errors
	///
	/// Returns [`ProviderError::Config`] when either required credential
	/// variable is missing.
	pub fn from_env() -> Result<Self> {
		Self::from_env_optional()?.ok_or_else(|| {
			ProviderError::Config(
				"AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set".to_string(),
			)
		})
	}

	/// Load AWS credentials from environment variables when present.
	///
	/// # Errors
	///
	/// Returns [`ProviderError::Config`] when only one required credential
	/// variable is present.
	pub fn from_env_optional() -> Result<Option<Self>> {
		let access_key_id = env::var("AWS_ACCESS_KEY_ID").ok();
		let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY").ok();
		let session_token = env::var("AWS_SESSION_TOKEN").ok();

		match (access_key_id, secret_access_key) {
			(Some(access_key_id), Some(secret_access_key)) => {
				let credentials = AwsCredentials {
					access_key_id,
					secret_access_key,
					session_token,
				};
				Ok(Some(credentials))
			}
			(None, None) => Ok(None),
			_ => Err(ProviderError::Config(
				"AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set together".to_string(),
			)),
		}
	}

	/// Return the AWS access key ID.
	#[must_use]
	pub fn access_key_id(&self) -> &str {
		&self.access_key_id
	}

	/// Return the AWS secret access key.
	#[must_use]
	pub fn secret_access_key(&self) -> &str {
		&self.secret_access_key
	}

	/// Return the AWS session token, when present.
	#[must_use]
	pub fn session_token(&self) -> Option<&str> {
		self.session_token.as_deref()
	}
}
