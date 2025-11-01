use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime};

/// CSRF token for form protection with advanced security features
#[derive(Debug, Clone)]
pub struct CsrfToken {
	token: String,
	created_at: SystemTime,
	max_age: Duration,
	rotation_enabled: bool,
}

impl CsrfToken {
	/// Generate a cryptographically secure CSRF token
	///
	/// Uses cryptographic random number generator and SHA-256 hashing
	/// to create a secure token.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfToken;
	///
	/// let token = CsrfToken::generate();
	/// assert!(token.token().len() > 0);
	/// assert!(!token.is_expired());
	/// ```
	pub fn generate() -> Self {
		let token = Self::generate_secure_token();
		Self {
			token,
			created_at: SystemTime::now(),
			max_age: Duration::from_secs(3600), // 1 hour default
			rotation_enabled: false,
		}
	}

	/// Generate a secure token with token rotation enabled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfToken;
	///
	/// let token = CsrfToken::generate_with_rotation();
	/// assert!(token.rotation_enabled());
	/// ```
	pub fn generate_with_rotation() -> Self {
		let token = Self::generate_secure_token();
		Self {
			token,
			created_at: SystemTime::now(),
			max_age: Duration::from_secs(3600),
			rotation_enabled: true,
		}
	}

	/// Generate a cryptographically secure random token
	fn generate_secure_token() -> String {
		let mut rng = rand::rng();
		let mut random_bytes = [0u8; 32];
		rng.fill_bytes(&mut random_bytes);

		// Hash the random bytes with SHA-256
		let mut hasher = Sha256::new();
		hasher.update(random_bytes);
		let hash_result = hasher.finalize();

		// Encode as base64
		general_purpose::STANDARD.encode(hash_result)
	}

	/// Create a new CSRF token with default settings
	///
	/// # Deprecated
	///
	/// Use `CsrfToken::generate()` instead for cryptographically secure tokens.
	pub fn new(secret: String) -> Self {
		Self {
			token: secret,
			created_at: SystemTime::now(),
			max_age: Duration::from_secs(3600), // 1 hour default
			rotation_enabled: false,
		}
	}

	/// Create a token with custom expiration time
	pub fn with_expiration(secret: String, max_age: Duration) -> Self {
		Self {
			token: secret,
			created_at: SystemTime::now(),
			max_age,
			rotation_enabled: false,
		}
	}

	/// Create a token with rotation enabled
	pub fn with_rotation(secret: String) -> Self {
		Self {
			token: secret,
			created_at: SystemTime::now(),
			max_age: Duration::from_secs(3600),
			rotation_enabled: true,
		}
	}

	/// Check if the token has expired
	pub fn is_expired(&self) -> bool {
		match self.created_at.elapsed() {
			Ok(elapsed) => elapsed > self.max_age,
			Err(_) => true, // Treat time errors as expired
		}
	}

	/// Get the token value
	pub fn token(&self) -> &str {
		&self.token
	}

	/// Rotate the token (generate a new one)
	///
	/// Generates a new cryptographically secure token and updates the creation time.
	/// This is useful for implementing token rotation policies.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfToken;
	///
	/// let mut token = CsrfToken::generate_with_rotation();
	/// let old_token = token.token().to_string();
	/// token.rotate();
	/// assert_ne!(old_token, token.token());
	/// ```
	pub fn rotate(&mut self) {
		self.token = Self::generate_secure_token();
		self.created_at = SystemTime::now();
	}

	/// Rotate the token with a custom value
	///
	/// This method is kept for backward compatibility.
	///
	/// # Deprecated
	///
	/// Use `rotate()` instead for cryptographically secure rotation.
	pub fn rotate_with_token(&mut self, new_token: String) {
		self.token = new_token;
		self.created_at = SystemTime::now();
	}

	/// Check if rotation is enabled
	pub fn rotation_enabled(&self) -> bool {
		self.rotation_enabled
	}

	/// Render as hidden input field
	pub fn as_hidden_input(&self) -> String {
		format!(
			r#"<input type="hidden" name="csrfmiddlewaretoken" value="{}" />"#,
			self.token
		)
	}

	/// Validate the token
	pub fn validate(&self, submitted_token: &str) -> Result<(), CsrfError> {
		if self.is_expired() {
			return Err(CsrfError::Expired);
		}

		if self.token != submitted_token {
			return Err(CsrfError::Invalid);
		}

		Ok(())
	}
}

impl Default for CsrfToken {
	fn default() -> Self {
		Self {
			token: "default-csrf-token".to_string(),
			created_at: SystemTime::now(),
			max_age: Duration::from_secs(3600),
			rotation_enabled: false,
		}
	}
}

/// CSRF validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsrfError {
	/// Token has expired
	Expired,
	/// Token is invalid
	Invalid,
	/// Token is missing
	Missing,
	/// Origin header mismatch
	OriginMismatch,
	/// Referer header mismatch
	RefererMismatch,
}

impl std::fmt::Display for CsrfError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CsrfError::Expired => write!(f, "CSRF token has expired"),
			CsrfError::Invalid => write!(f, "CSRF token is invalid"),
			CsrfError::Missing => write!(f, "CSRF token is missing"),
			CsrfError::OriginMismatch => write!(f, "Origin header does not match"),
			CsrfError::RefererMismatch => write!(f, "Referer header does not match"),
		}
	}
}

impl std::error::Error for CsrfError {}

/// SameSite cookie attribute for CSRF protection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
	/// Strict mode: cookies are only sent in first-party context
	Strict,
	/// Lax mode: cookies are sent with top-level navigation
	Lax,
	/// None mode: cookies are sent in all contexts (requires Secure)
	None,
}

impl SameSite {
	/// Convert to cookie attribute string
	pub fn as_str(&self) -> &str {
		match self {
			SameSite::Strict => "Strict",
			SameSite::Lax => "Lax",
			SameSite::None => "None",
		}
	}
}

/// CSRF validator with advanced security features
#[derive(Debug, Clone)]
pub struct CsrfValidator {
	check_origin: bool,
	check_referer: bool,
	trusted_origins: Vec<String>,
	same_site: SameSite,
	secure_only: bool,
}

impl CsrfValidator {
	/// Create a new validator with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfValidator;
	///
	/// let validator = CsrfValidator::new();
	/// assert!(validator.is_origin_check_enabled());
	/// ```
	pub fn new() -> Self {
		Self {
			check_origin: true,
			check_referer: true,
			trusted_origins: Vec::new(),
			same_site: SameSite::Lax,
			secure_only: true,
		}
	}

	/// Add a trusted origin
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfValidator;
	///
	/// let validator = CsrfValidator::new()
	///     .add_trusted_origin("https://example.com".to_string())
	///     .add_trusted_origin("https://api.example.com".to_string());
	/// ```
	pub fn add_trusted_origin(mut self, origin: String) -> Self {
		self.trusted_origins.push(origin);
		self
	}

	/// Enable/disable origin check
	pub fn check_origin(mut self, enabled: bool) -> Self {
		self.check_origin = enabled;
		self
	}

	/// Enable/disable referer check
	pub fn check_referer(mut self, enabled: bool) -> Self {
		self.check_referer = enabled;
		self
	}

	/// Set SameSite cookie attribute
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{CsrfValidator, SameSite};
	///
	/// let validator = CsrfValidator::new()
	///     .with_same_site(SameSite::Strict);
	/// ```
	pub fn with_same_site(mut self, same_site: SameSite) -> Self {
		self.same_site = same_site;
		self
	}

	/// Set secure-only flag
	///
	/// When true, cookies are only sent over HTTPS connections.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfValidator;
	///
	/// let validator = CsrfValidator::new()
	///     .with_secure_only(true);
	/// ```
	pub fn with_secure_only(mut self, secure: bool) -> Self {
		self.secure_only = secure;
		self
	}

	/// Get the SameSite attribute
	pub fn same_site(&self) -> SameSite {
		self.same_site
	}

	/// Check if secure-only is enabled
	pub fn is_secure_only(&self) -> bool {
		self.secure_only
	}

	/// Check if origin checking is enabled
	pub fn is_origin_check_enabled(&self) -> bool {
		self.check_origin
	}

	/// Generate cookie attributes string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfValidator;
	///
	/// let validator = CsrfValidator::new();
	/// let attrs = validator.cookie_attributes();
	/// assert!(attrs.contains("SameSite=Lax"));
	/// assert!(attrs.contains("Secure"));
	/// ```
	pub fn cookie_attributes(&self) -> String {
		let mut attrs = vec![format!("SameSite={}", self.same_site.as_str())];

		if self.secure_only {
			attrs.push("Secure".to_string());
		}

		// Always set HttpOnly for CSRF cookies
		attrs.push("HttpOnly".to_string());

		attrs.join("; ")
	}

	/// Validate origin header with enhanced security checks
	///
	/// Performs comprehensive origin validation including:
	/// - Checks if origin is present
	/// - Validates against expected origin
	/// - Checks trusted origins list
	/// - Ensures origin uses HTTPS when secure_only is enabled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::CsrfValidator;
	///
	/// let validator = CsrfValidator::new()
	///     .add_trusted_origin("https://example.com".to_string());
	///
	/// // Valid origin
	/// assert!(validator.validate_origin(
	///     Some("https://example.com"),
	///     "https://example.com"
	/// ).is_ok());
	///
	/// // Invalid origin
	/// assert!(validator.validate_origin(
	///     Some("https://evil.com"),
	///     "https://example.com"
	/// ).is_err());
	/// ```
	pub fn validate_origin(&self, origin: Option<&str>, expected: &str) -> Result<(), CsrfError> {
		if !self.check_origin {
			return Ok(());
		}

		let origin = origin.ok_or(CsrfError::OriginMismatch)?;

		// Check if secure_only is enabled and origin uses HTTPS
		if self.secure_only && !origin.starts_with("https://") {
			return Err(CsrfError::OriginMismatch);
		}

		// Normalize origins for comparison (remove trailing slashes)
		let origin_normalized = origin.trim_end_matches('/');
		let expected_normalized = expected.trim_end_matches('/');

		// Check if origin matches expected or is in trusted list
		if origin_normalized == expected_normalized
			|| self
				.trusted_origins
				.iter()
				.any(|t| t.trim_end_matches('/') == origin_normalized)
		{
			Ok(())
		} else {
			Err(CsrfError::OriginMismatch)
		}
	}

	/// Validate referer header
	pub fn validate_referer(
		&self,
		referer: Option<&str>,
		expected_host: &str,
	) -> Result<(), CsrfError> {
		if !self.check_referer {
			return Ok(());
		}

		let referer = referer.ok_or(CsrfError::RefererMismatch)?;

		if referer.contains(expected_host) {
			Ok(())
		} else {
			Err(CsrfError::RefererMismatch)
		}
	}
}

impl Default for CsrfValidator {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::thread::sleep;

	#[test]
	fn test_csrf_token_creation() {
		let token = CsrfToken::new("test-token".to_string());
		assert_eq!(token.token(), "test-token");
		assert!(!token.is_expired());
	}

	#[test]
	fn test_csrf_token_expiration() {
		let token =
			CsrfToken::with_expiration("test-token".to_string(), Duration::from_millis(100));
		assert!(!token.is_expired());

		sleep(Duration::from_millis(150));
		assert!(token.is_expired());
	}

	#[test]
	fn test_csrf_token_validation() {
		let token = CsrfToken::new("test-token".to_string());

		assert!(token.validate("test-token").is_ok());
		assert_eq!(token.validate("wrong-token"), Err(CsrfError::Invalid));
	}

	#[test]
	fn test_csrf_token_rotation() {
		let mut token = CsrfToken::generate_with_rotation();
		assert!(token.rotation_enabled());

		let old_token = token.token().to_string();
		token.rotate();
		assert_ne!(token.token(), old_token);
	}

	#[test]
	fn test_csrf_token_generation() {
		let token1 = CsrfToken::generate();
		let token2 = CsrfToken::generate();

		// Tokens should be different
		assert_ne!(token1.token(), token2.token());

		// Tokens should be base64 encoded (64 characters for SHA-256)
		assert_eq!(token1.token().len(), 44); // Base64 encoded 32 bytes
	}

	#[test]
	fn test_csrf_validator_same_site() {
		let validator = CsrfValidator::new().with_same_site(SameSite::Strict);

		assert_eq!(validator.same_site(), SameSite::Strict);

		let attrs = validator.cookie_attributes();
		assert!(attrs.contains("SameSite=Strict"));
		assert!(attrs.contains("Secure"));
		assert!(attrs.contains("HttpOnly"));
	}

	#[test]
	fn test_csrf_validator_secure_only() {
		let validator = CsrfValidator::new().with_secure_only(false);

		assert!(!validator.is_secure_only());

		let attrs = validator.cookie_attributes();
		assert!(!attrs.contains("Secure"));
	}

	#[test]
	fn test_csrf_validator_origin_https_enforcement() {
		let validator = CsrfValidator::new(); // secure_only is true by default

		// HTTP origin should be rejected
		assert!(
			validator
				.validate_origin(Some("http://example.com"), "https://example.com")
				.is_err()
		);

		// HTTPS origin should be accepted
		assert!(
			validator
				.validate_origin(Some("https://example.com"), "https://example.com")
				.is_ok()
		);
	}

	#[test]
	fn test_csrf_validator_origin_normalization() {
		let validator = CsrfValidator::new();

		// Origins with trailing slashes should match
		assert!(
			validator
				.validate_origin(Some("https://example.com/"), "https://example.com")
				.is_ok()
		);

		assert!(
			validator
				.validate_origin(Some("https://example.com"), "https://example.com/")
				.is_ok()
		);
	}

	#[test]
	fn test_csrf_validator_origin() {
		let validator = CsrfValidator::new().add_trusted_origin("https://example.com".to_string());

		assert!(
			validator
				.validate_origin(Some("https://example.com"), "https://example.com")
				.is_ok()
		);
		assert_eq!(
			validator.validate_origin(Some("https://evil.com"), "https://example.com"),
			Err(CsrfError::OriginMismatch)
		);
		assert_eq!(
			validator.validate_origin(None, "https://example.com"),
			Err(CsrfError::OriginMismatch)
		);
	}

	#[test]
	fn test_csrf_validator_referer() {
		let validator = CsrfValidator::new();

		assert!(
			validator
				.validate_referer(Some("https://example.com/page"), "example.com")
				.is_ok()
		);
		assert_eq!(
			validator.validate_referer(Some("https://evil.com/page"), "example.com"),
			Err(CsrfError::RefererMismatch)
		);
		assert_eq!(
			validator.validate_referer(None, "example.com"),
			Err(CsrfError::RefererMismatch)
		);
	}

	#[test]
	fn test_csrf_validator_disabled_checks() {
		let validator = CsrfValidator::new()
			.check_origin(false)
			.check_referer(false);

		assert!(
			validator
				.validate_origin(Some("https://evil.com"), "https://example.com")
				.is_ok()
		);
		assert!(
			validator
				.validate_referer(Some("https://evil.com/page"), "example.com")
				.is_ok()
		);
	}

	#[test]
	fn test_csrf_expired_token_validation() {
		let token =
			CsrfToken::with_expiration("test-token".to_string(), Duration::from_millis(100));

		sleep(Duration::from_millis(150));
		assert_eq!(token.validate("test-token"), Err(CsrfError::Expired));
	}
}
