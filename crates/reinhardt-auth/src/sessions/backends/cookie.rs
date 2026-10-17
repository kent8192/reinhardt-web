//! Cookie-based session backend with encryption and signing
//!
//! This module provides session storage in encrypted, signed cookies.
//! Sessions are stored entirely on the client side, eliminating the need for server-side storage.
//!
//! ## Features
//!
//! - **AES-GCM encryption**: Session data is encrypted using AES-256-GCM
//! - **HMAC signing**: Encrypted data is signed to prevent tampering
//! - **Size limitation**: Handles cookie size limits (max 4096 bytes)
//! - **Secure**: Protects against tampering and eavesdropping
//! - **Expiry**: The TTL passed to `save` is sealed into the signed payload as an
//!   absolute expiry; once it passes, `load` returns `None`, `exists` returns
//!   `false`, and the stored session is removed
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_auth::sessions::backends::{CookieSessionBackend, SessionBackend};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a cookie backend with encryption key
//! let encryption_key = b"32_byte_encryption_key_here_1234";
//! let signing_secret = b"32_byte_signing_secret_here_5678";
//! let backend = CookieSessionBackend::new(encryption_key, signing_secret);
//!
//! // Store session data
//! let session_data = json!({
//!     "user_id": 42,
//!     "username": "alice",
//! });
//!
//! backend.save("session_key", &session_data, None).await?;
//! # Ok(())
//! # }
//! ```

use super::{SessionBackend, SessionError};
use aes_gcm::{
	Aes256Gcm, Nonce,
	aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Maximum cookie size in bytes (4KB)
const MAX_COOKIE_SIZE: usize = 4096;

/// Nonce size for AES-GCM (96 bits / 12 bytes)
const NONCE_SIZE: usize = 12;

/// Plaintext sealed inside each encrypted, signed session value
///
/// The expiry is part of the authenticated payload, so changing it fails
/// signature verification.
#[derive(Serialize, Deserialize)]
struct StoredSession {
	data: serde_json::Value,
	/// Absolute expiry in milliseconds since the Unix epoch; `None` never expires
	expires_at_ms: Option<u64>,
}

/// Current wall-clock time in milliseconds since the Unix epoch
fn now_unix_millis() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map_or(0, |elapsed| {
			u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
		})
}

/// Absolute expiry for a TTL in seconds, saturating instead of overflowing
fn expiry_from_ttl(ttl: Option<u64>, now_ms: u64) -> Option<u64> {
	ttl.map(|secs| now_ms.saturating_add(secs.saturating_mul(1000)))
}

/// A session expires once the current time is past its expiry, as in the
/// other session backends
fn is_expired(expires_at_ms: Option<u64>, now_ms: u64) -> bool {
	expires_at_ms.is_some_and(|expires_at| now_ms > expires_at)
}

/// Cookie session backend configuration
///
/// This backend encrypts and signs session data for storage in cookies.
///
/// ## Example
///
/// ```rust
/// use reinhardt_auth::sessions::backends::{CookieSessionBackend, SessionBackend};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let encryption_key = b"32_byte_encryption_key_here_1234";
/// let signing_secret = b"32_byte_signing_secret_here_5678";
/// let backend = CookieSessionBackend::new(encryption_key, signing_secret);
///
/// let data = json!({"user_id": 123});
/// backend.save("session_abc", &data, None).await?;
///
/// let loaded: Option<serde_json::Value> = backend.load("session_abc").await?;
/// assert_eq!(loaded.unwrap()["user_id"], 123);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CookieSessionBackend {
	cipher: Arc<Aes256Gcm>,
	signing_secret: Arc<Vec<u8>>,
	// In-memory storage for demonstration (maps session_key -> encrypted_data)
	storage: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}

impl CookieSessionBackend {
	/// Create a new cookie session backend
	///
	/// # Arguments
	///
	/// * `encryption_key` - 32-byte key for AES-256-GCM encryption
	/// * `signing_secret` - Secret key for HMAC-SHA256 signing
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_auth::sessions::backends::CookieSessionBackend;
	///
	/// let encryption_key = b"32_byte_encryption_key_here_1234";
	/// let signing_secret = b"32_byte_signing_secret_here_5678";
	/// let backend = CookieSessionBackend::new(encryption_key, signing_secret);
	/// ```
	pub fn new(encryption_key: &[u8; 32], signing_secret: &[u8]) -> Self {
		let cipher = Aes256Gcm::new(encryption_key.into());
		Self {
			cipher: Arc::new(cipher),
			signing_secret: Arc::new(signing_secret.to_vec()),
			storage: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
		}
	}

	/// Encrypt and sign session data
	///
	/// Returns base64-encoded string: `base64(nonce || encrypted_data || signature)`
	fn encrypt_and_sign(&self, data: &[u8]) -> Result<String, SessionError> {
		// Generate random nonce
		let mut nonce_bytes = [0u8; NONCE_SIZE];
		OsRng.fill_bytes(&mut nonce_bytes);
		let nonce = Nonce::from(nonce_bytes);

		// Encrypt data
		let encrypted = self
			.cipher
			.encrypt(&nonce, data)
			.map_err(|e| SessionError::SerializationError(format!("Encryption failed: {}", e)))?;

		// Combine nonce and encrypted data
		let mut combined = Vec::with_capacity(NONCE_SIZE + encrypted.len());
		combined.extend_from_slice(&nonce_bytes);
		combined.extend_from_slice(&encrypted);

		// Sign the combined data
		let signature = self.sign(&combined)?;

		// Append signature
		combined.extend_from_slice(&signature);

		// Encode as base64
		Ok(BASE64.encode(&combined))
	}

	/// Verify signature and decrypt session data
	///
	/// Input format: `base64(nonce || encrypted_data || signature)`
	fn verify_and_decrypt(&self, encoded: &str) -> Result<Vec<u8>, SessionError> {
		// Decode base64
		let combined = BASE64.decode(encoded).map_err(|e| {
			SessionError::SerializationError(format!("Base64 decode failed: {}", e))
		})?;

		// Check minimum size (nonce + encrypted_data + signature)
		if combined.len() < NONCE_SIZE + 32 {
			return Err(SessionError::SerializationError(
				"Invalid cookie data: too short".to_string(),
			));
		}

		// Split signature from the rest
		let (data_with_nonce, signature) = combined.split_at(combined.len() - 32);

		// Verify signature
		self.verify_signature(data_with_nonce, signature)?;

		// Extract nonce and encrypted data
		let (nonce_bytes, encrypted) = data_with_nonce.split_at(NONCE_SIZE);
		// SAFETY: We checked the length above, nonce_bytes is exactly NONCE_SIZE bytes
		let nonce_array: [u8; NONCE_SIZE] = nonce_bytes
			.try_into()
			.map_err(|_| SessionError::SerializationError("Invalid nonce size".to_string()))?;
		let nonce = Nonce::from(nonce_array);

		// Decrypt data
		let decrypted = self
			.cipher
			.decrypt(&nonce, encrypted)
			.map_err(|e| SessionError::SerializationError(format!("Decryption failed: {}", e)))?;

		Ok(decrypted)
	}

	/// Sign data using HMAC-SHA256
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SessionError> {
		let mut mac = <HmacSha256 as hmac::Mac>::new_from_slice(&self.signing_secret)
			.map_err(|e| SessionError::SerializationError(format!("HMAC init failed: {}", e)))?;
		mac.update(data);
		Ok(mac.finalize().into_bytes().to_vec())
	}

	/// Verify HMAC-SHA256 signature
	fn verify_signature(&self, data: &[u8], signature: &[u8]) -> Result<(), SessionError> {
		let mut mac = <HmacSha256 as hmac::Mac>::new_from_slice(&self.signing_secret)
			.map_err(|e| SessionError::SerializationError(format!("HMAC init failed: {}", e)))?;
		mac.update(data);
		mac.verify_slice(signature).map_err(|_| {
			SessionError::SerializationError("Signature verification failed".to_string())
		})?;
		Ok(())
	}

	/// Load the stored session for `session_key`, treating an expired one as missing
	///
	/// An expired value is removed only if it is still the value that was read,
	/// so a concurrent `save` for the same key is not discarded.
	async fn load_unexpired(
		&self,
		session_key: &str,
	) -> Result<Option<StoredSession>, SessionError> {
		let Some(encoded) = self.storage.read().await.get(session_key).cloned() else {
			return Ok(None);
		};

		// Verify and decrypt
		let decrypted = self.verify_and_decrypt(&encoded)?;

		// Deserialize
		let stored: StoredSession = serde_json::from_slice(&decrypted).map_err(|e| {
			SessionError::SerializationError(format!("Deserialization failed: {}", e))
		})?;

		if is_expired(stored.expires_at_ms, now_unix_millis()) {
			let mut storage = self.storage.write().await;
			if storage.get(session_key) == Some(&encoded) {
				storage.remove(session_key);
			}
			return Ok(None);
		}

		Ok(Some(stored))
	}
}

#[async_trait]
impl SessionBackend for CookieSessionBackend {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		let Some(stored) = self.load_unexpired(session_key).await? else {
			return Ok(None);
		};

		let data: T = serde_json::from_value(stored.data).map_err(|e| {
			SessionError::SerializationError(format!("Deserialization failed: {}", e))
		})?;

		Ok(Some(data))
	}

	async fn save<T>(
		&self,
		session_key: &str,
		data: &T,
		ttl: Option<u64>,
	) -> Result<(), SessionError>
	where
		T: Serialize + Send + Sync,
	{
		// Serialize data together with its absolute expiry
		let stored = StoredSession {
			data: serde_json::to_value(data).map_err(|e| {
				SessionError::SerializationError(format!("Serialization failed: {}", e))
			})?,
			expires_at_ms: expiry_from_ttl(ttl, now_unix_millis()),
		};
		let json_bytes = serde_json::to_vec(&stored).map_err(|e| {
			SessionError::SerializationError(format!("Serialization failed: {}", e))
		})?;

		// Encrypt and sign
		let encoded = self.encrypt_and_sign(&json_bytes)?;

		// Check size limitation
		if encoded.len() > MAX_COOKIE_SIZE {
			return Err(SessionError::SerializationError(format!(
				"Session data too large: {} bytes (max {} bytes)",
				encoded.len(),
				MAX_COOKIE_SIZE
			)));
		}

		// Store in memory (HTTP cookie handling via middleware layer)
		let mut storage = self.storage.write().await;
		storage.insert(session_key.to_string(), encoded);

		Ok(())
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		let mut storage = self.storage.write().await;
		storage.remove(session_key);
		Ok(())
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		Ok(self.load_unexpired(session_key).await?.is_some())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde_json::json;
	use std::time::Duration;

	const TEST_ENCRYPTION_KEY: &[u8; 32] = b"test_encryption_key_32bytes_ok!!";
	const TEST_SIGNING_SECRET: &[u8] = b"test_signing_secret_key";

	/// Store a sealed session with an explicit expiry, bypassing `save`
	async fn insert_stored_session(
		backend: &CookieSessionBackend,
		session_key: &str,
		expires_at_ms: Option<u64>,
	) {
		let stored = StoredSession {
			data: json!({"user_id": 7}),
			expires_at_ms,
		};
		let encoded = backend
			.encrypt_and_sign(&serde_json::to_vec(&stored).unwrap())
			.unwrap();
		backend
			.storage
			.write()
			.await
			.insert(session_key.to_string(), encoded);
	}

	#[rstest]
	#[tokio::test]
	async fn test_save_with_ttl_is_rejected_after_expiry() {
		// Arrange
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		backend
			.save("ttl_session", &json!({"user_id": 42}), Some(1))
			.await
			.unwrap();
		let before_expiry: Option<serde_json::Value> = backend.load("ttl_session").await.unwrap();

		// Act
		tokio::time::sleep(Duration::from_millis(1_100)).await;
		let after_expiry: Option<serde_json::Value> = backend.load("ttl_session").await.unwrap();
		let exists_after_expiry = backend.exists("ttl_session").await.unwrap();

		// Assert
		assert_eq!(before_expiry, Some(json!({"user_id": 42})));
		assert_eq!(after_expiry, None);
		assert!(!exists_after_expiry);
		assert!(!backend.storage.read().await.contains_key("ttl_session"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_save_with_unexpired_ttl_loads() {
		// Arrange
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		backend
			.save("ttl_session", &json!({"user_id": 42}), Some(3600))
			.await
			.unwrap();

		// Act
		let loaded: Option<serde_json::Value> = backend.load("ttl_session").await.unwrap();
		let exists = backend.exists("ttl_session").await.unwrap();

		// Assert
		assert_eq!(loaded, Some(json!({"user_id": 42})));
		assert!(exists);
	}

	#[rstest]
	#[tokio::test]
	async fn test_load_removes_expired_session() {
		// Arrange
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		insert_stored_session(&backend, "expired", Some(now_unix_millis() - 1_000)).await;

		// Act
		let loaded: Option<serde_json::Value> = backend.load("expired").await.unwrap();

		// Assert
		assert_eq!(loaded, None);
		assert!(!backend.storage.read().await.contains_key("expired"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_exists_removes_expired_session() {
		// Arrange
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		insert_stored_session(&backend, "expired", Some(now_unix_millis() - 1_000)).await;

		// Act
		let exists = backend.exists("expired").await.unwrap();

		// Assert
		assert!(!exists);
		assert!(!backend.storage.read().await.contains_key("expired"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_session_without_expiry_loads() {
		// Arrange
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		insert_stored_session(&backend, "no_expiry", None).await;

		// Act
		let loaded: Option<serde_json::Value> = backend.load("no_expiry").await.unwrap();

		// Assert
		assert_eq!(loaded, Some(json!({"user_id": 7})));
	}

	#[rstest]
	#[case::no_expiry(None, u64::MAX, false)]
	#[case::before_expiry(Some(1_000), 999, false)]
	#[case::at_expiry(Some(1_000), 1_000, false)]
	#[case::past_expiry(Some(1_000), 1_001, true)]
	fn test_is_expired_boundary(
		#[case] expires_at_ms: Option<u64>,
		#[case] now_ms: u64,
		#[case] expected: bool,
	) {
		// Act
		let expired = is_expired(expires_at_ms, now_ms);

		// Assert
		assert_eq!(expired, expected);
	}

	#[rstest]
	#[case::no_ttl(None, 5_000, None)]
	#[case::zero_ttl(Some(0), 5_000, Some(5_000))]
	#[case::finite_ttl(Some(60), 5_000, Some(65_000))]
	#[case::overflowing_ttl(Some(u64::MAX), 5_000, Some(u64::MAX))]
	fn test_expiry_from_ttl(
		#[case] ttl: Option<u64>,
		#[case] now_ms: u64,
		#[case] expected: Option<u64>,
	) {
		// Act
		let expires_at_ms = expiry_from_ttl(ttl, now_ms);

		// Assert
		assert_eq!(expires_at_ms, expected);
	}

	#[tokio::test]
	async fn test_cookie_backend_save_and_load() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let session_key = "test_session_123";

		let data = json!({
			"user_id": 42,
			"username": "alice",
			"is_admin": true,
		});

		backend.save(session_key, &data, None).await.unwrap();

		let loaded: Option<serde_json::Value> = backend.load(session_key).await.unwrap();
		let loaded_data = loaded.unwrap();
		assert_eq!(loaded_data["user_id"], 42);
		assert_eq!(loaded_data["username"], "alice");
		assert_eq!(loaded_data["is_admin"], true);
	}

	#[tokio::test]
	async fn test_cookie_backend_delete() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let session_key = "test_session_456";

		let data = json!({"key": "value"});
		backend.save(session_key, &data, None).await.unwrap();

		assert!(backend.exists(session_key).await.unwrap());

		backend.delete(session_key).await.unwrap();

		assert!(!backend.exists(session_key).await.unwrap());
		let loaded: Option<serde_json::Value> = backend.load(session_key).await.unwrap();
		assert!(loaded.is_none());
	}

	#[tokio::test]
	async fn test_encryption_and_decryption() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);

		let original_data = b"Hello, World! This is a test message.";
		let encrypted = backend.encrypt_and_sign(original_data).unwrap();

		// Verify it's encrypted (not the same as original)
		assert_ne!(encrypted, BASE64.encode(original_data));

		let decrypted = backend.verify_and_decrypt(&encrypted).unwrap();
		assert_eq!(decrypted, original_data);
	}

	#[tokio::test]
	async fn test_signature_verification() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let data = b"test data for signing";

		let signature = backend.sign(data).unwrap();
		assert_eq!(signature.len(), 32); // SHA256 produces 32 bytes

		backend.verify_signature(data, &signature).unwrap();
	}

	#[tokio::test]
	async fn test_tampered_signature_fails() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);

		let data = b"original data";
		let encrypted = backend.encrypt_and_sign(data).unwrap();

		// Tamper with the encrypted data
		let mut tampered = BASE64.decode(&encrypted).unwrap();
		tampered[0] ^= 0xFF; // Flip bits in first byte
		let tampered_encoded = BASE64.encode(&tampered);

		// Verification should fail
		let result = backend.verify_and_decrypt(&tampered_encoded);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_size_limitation() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let session_key = "large_session";

		// Create large data that exceeds cookie size limit
		let large_string = "x".repeat(5000);
		let large_data = json!({
			"large_field": large_string,
		});

		let result = backend.save(session_key, &large_data, None).await;
		assert!(result.is_err());

		match result {
			Err(SessionError::SerializationError(msg)) => {
				assert!(msg.contains("too large"));
				assert!(msg.contains("4096"));
			}
			_ => panic!("Expected SerializationError for oversized data"),
		}
	}

	#[tokio::test]
	async fn test_cookie_backend_exists() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);

		assert!(!backend.exists("nonexistent").await.unwrap());

		let data = json!({"test": "data"});
		backend.save("existing", &data, None).await.unwrap();

		assert!(backend.exists("existing").await.unwrap());
	}

	#[tokio::test]
	async fn test_different_encryption_keys_produce_different_results() {
		let key1 = b"first_encryption_key_32_bytes_12";
		let key2 = b"second_encryption_key_32bytes_34";
		let secret = b"shared_signing_secret";

		let backend1 = CookieSessionBackend::new(key1, secret);
		let backend2 = CookieSessionBackend::new(key2, secret);

		let data = b"same data";
		let encrypted1 = backend1.encrypt_and_sign(data).unwrap();
		let encrypted2 = backend2.encrypt_and_sign(data).unwrap();

		// Different keys should produce different encrypted output
		assert_ne!(encrypted1, encrypted2);

		// Backend1 can decrypt its own data
		assert!(backend1.verify_and_decrypt(&encrypted1).is_ok());

		// Backend2 cannot decrypt Backend1's data (wrong key)
		assert!(backend2.verify_and_decrypt(&encrypted1).is_err());
	}

	#[tokio::test]
	async fn test_empty_session_data() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let session_key = "empty_session";

		let empty_data: HashMap<String, serde_json::Value> = HashMap::new();
		backend.save(session_key, &empty_data, None).await.unwrap();

		let loaded: Option<HashMap<String, serde_json::Value>> =
			backend.load(session_key).await.unwrap();
		assert!(loaded.is_some());
		assert_eq!(loaded.unwrap().len(), 0);
	}

	#[tokio::test]
	async fn test_complex_nested_data() {
		let backend = CookieSessionBackend::new(TEST_ENCRYPTION_KEY, TEST_SIGNING_SECRET);
		let session_key = "complex_session";

		let complex_data = json!({
			"user": {
				"id": 123,
				"name": "Alice",
				"roles": ["admin", "user"],
				"metadata": {
					"created_at": "2024-01-01",
					"last_login": "2024-01-15",
				}
			},
			"settings": {
				"theme": "dark",
				"language": "en",
			}
		});

		backend
			.save(session_key, &complex_data, None)
			.await
			.unwrap();

		let loaded: Option<serde_json::Value> = backend.load(session_key).await.unwrap();
		let loaded_data = loaded.unwrap();
		assert_eq!(loaded_data["user"]["id"], 123);
		assert_eq!(loaded_data["user"]["roles"][0], "admin");
		assert_eq!(loaded_data["settings"]["theme"], "dark");
	}
}
