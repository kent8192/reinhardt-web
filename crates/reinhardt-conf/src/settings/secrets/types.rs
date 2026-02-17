//! Secret types that prevent accidental exposure
//!
//! These types wrap sensitive data and ensure it's not accidentally logged
//! or displayed in debug output.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone)]
pub enum SecretError {
	NotFound(String),
	Provider(String),
	ProviderError(String),
	NetworkError(String),
}

impl fmt::Display for SecretError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SecretError::NotFound(msg) => write!(f, "Secret not found: {}", msg),
			SecretError::Provider(msg) => write!(f, "Provider error: {}", msg),
			SecretError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
			SecretError::NetworkError(msg) => write!(f, "Network error: {}", msg),
		}
	}
}

impl std::error::Error for SecretError {}

pub type SecretResult<T> = Result<T, SecretError>;

pub struct SecretManager;
#[derive(Debug, Clone, Default)]
pub struct SecretMetadata {
	pub created_at: Option<chrono::DateTime<chrono::Utc>>,
	pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
pub struct SecretVersion;

#[async_trait]
pub trait SecretProvider: Send + Sync {
	async fn get_secret(&self, name: &str) -> SecretResult<SecretString>;
	async fn get_secret_with_metadata(
		&self,
		name: &str,
	) -> SecretResult<(SecretString, SecretMetadata)>;
	async fn set_secret(&self, name: &str, value: SecretString) -> SecretResult<()>;
	async fn delete_secret(&self, name: &str) -> SecretResult<()>;
	async fn list_secrets(&self) -> SecretResult<Vec<String>>;
	fn exists(&self, name: &str) -> bool;
	fn name(&self) -> &str;
}

/// A secret string that won't be exposed in logs or debug output
#[derive(Clone, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecretString {
	#[serde(rename = "secret")]
	inner: String,
}

impl Serialize for SecretString {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		// Always serialize as [REDACTED] to prevent accidental exposure
		serializer.serialize_str("[REDACTED]")
	}
}

impl SecretString {
	/// Create a new secret string
	pub fn new(secret: impl Into<String>) -> Self {
		Self {
			inner: secret.into(),
		}
	}
	/// Access the secret value (use with caution)
	///
	pub fn expose_secret(&self) -> &str {
		&self.inner
	}
	/// Convert to owned String (consumes self)
	///
	pub fn into_inner(self) -> String {
		self.inner.clone()
	}
	/// Get the length of the secret
	///
	pub fn len(&self) -> usize {
		self.inner.len()
	}
	/// Check if the secret is empty
	///
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

impl fmt::Debug for SecretString {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("SecretString([REDACTED])")
	}
}

impl fmt::Display for SecretString {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("[REDACTED]")
	}
}

impl From<String> for SecretString {
	fn from(s: String) -> Self {
		Self::new(s)
	}
}

impl From<&str> for SecretString {
	fn from(s: &str) -> Self {
		Self::new(s.to_string())
	}
}

impl PartialEq for SecretString {
	fn eq(&self, other: &Self) -> bool {
		// Use constant-time comparison to prevent timing attacks
		use subtle::ConstantTimeEq;
		self.inner.as_bytes().ct_eq(other.inner.as_bytes()).into()
	}
}

impl Eq for SecretString {}

/// A generic secret value that can hold any serializable type
#[derive(Clone, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecretValue<T: Zeroize> {
	#[serde(bound(deserialize = "T: Deserialize<'de>"))]
	inner: T,
}

impl<T: Zeroize> Serialize for SecretValue<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		// Always serialize as [REDACTED] to prevent accidental exposure
		serializer.serialize_str("[REDACTED]")
	}
}

impl<T: Zeroize> SecretValue<T> {
	/// Create a new secret value
	pub fn new(value: T) -> Self {
		Self { inner: value }
	}
	/// Access the secret value (use with caution)
	///
	pub fn expose_secret(&self) -> &T {
		&self.inner
	}
	/// Convert to owned value (consumes self)
	///
	pub fn into_inner(self) -> T
	where
		T: Clone,
	{
		self.inner.clone()
	}
}

impl<T: Zeroize> fmt::Debug for SecretValue<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("SecretValue([REDACTED])")
	}
}

impl<T: Zeroize> fmt::Display for SecretValue<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("[REDACTED]")
	}
}

impl<T: Zeroize> From<T> for SecretValue<T> {
	fn from(value: T) -> Self {
		Self::new(value)
	}
}

impl<T: Zeroize + PartialEq> PartialEq for SecretValue<T> {
	fn eq(&self, other: &Self) -> bool {
		self.inner == other.inner
	}
}

impl<T: Zeroize + Eq> Eq for SecretValue<T> {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_secret_string_debug() {
		let secret = SecretString::new("my-secret-password");
		let debug_output = format!("{:?}", secret);
		assert!(!debug_output.contains("my-secret-password"));
		assert!(debug_output.contains("REDACTED"));
	}

	#[test]
	fn test_secret_string_display() {
		let secret = SecretString::new("my-secret-password");
		let display_output = format!("{}", secret);
		assert!(!display_output.contains("my-secret-password"));
		assert!(display_output.contains("REDACTED"));
	}

	#[test]
	fn test_secret_string_expose() {
		let secret = SecretString::new("my-secret-password");
		assert_eq!(secret.expose_secret(), "my-secret-password");
	}

	#[test]
	fn test_secret_string_len() {
		let secret = SecretString::new("password");
		assert_eq!(secret.len(), 8);
		assert!(!secret.is_empty());

		let empty = SecretString::new("");
		assert_eq!(empty.len(), 0);
		assert!(empty.is_empty());
	}

	#[test]
	fn test_secret_string_equality() {
		let secret1 = SecretString::new("password");
		let secret2 = SecretString::new("password");
		let secret3 = SecretString::new("different");

		assert_eq!(secret1, secret2);
		assert_ne!(secret1, secret3);
	}

	#[test]
	fn test_secret_value_debug() {
		let secret = SecretValue::new(12345);
		let debug_output = format!("{:?}", secret);
		assert!(!debug_output.contains("12345"));
		assert!(debug_output.contains("REDACTED"));
	}

	#[test]
	fn test_secret_value_expose() {
		let secret = SecretValue::new(vec![1, 2, 3, 4, 5]);
		assert_eq!(secret.expose_secret(), &vec![1, 2, 3, 4, 5]);
	}

	#[test]
	fn test_secret_string_serialization_redacts_value() {
		let secret = SecretString::new("my-super-secret-password");
		let json = serde_json::to_string(&secret).unwrap();
		// Serialization should always output [REDACTED], not the actual secret
		assert!(!json.contains("my-super-secret-password"));
		assert!(json.contains("[REDACTED]"));
		assert_eq!(json, "\"[REDACTED]\"");
	}

	#[test]
	fn test_secret_string_deserialization() {
		// Deserialization should still work for config loading
		let json = r#"{"secret":"test-secret"}"#;
		let deserialized: SecretString = serde_json::from_str(json).unwrap();
		assert_eq!(deserialized.expose_secret(), "test-secret");
	}

	#[test]
	fn test_secret_value_serialization_redacts_value() {
		let secret = SecretValue::new(42);
		let json = serde_json::to_string(&secret).unwrap();
		// Serialization should always output [REDACTED], not the actual value
		assert!(!json.contains("42"));
		assert!(json.contains("[REDACTED]"));
		assert_eq!(json, "\"[REDACTED]\"");
	}

	#[test]
	fn test_secret_value_deserialization() {
		// Deserialization should still work for config loading
		let json = r#"{"inner":42}"#;
		let deserialized: SecretValue<i32> = serde_json::from_str(json).unwrap();
		assert_eq!(*deserialized.expose_secret(), 42);
	}
}
