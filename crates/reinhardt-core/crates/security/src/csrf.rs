//! CSRF (Cross-Site Request Forgery) protection

use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

/// CSRF token length (64 characters)
pub const CSRF_TOKEN_LENGTH: usize = 64;

/// CSRF secret length (32 characters)
pub const CSRF_SECRET_LENGTH: usize = 32;

/// Allowed characters for CSRF tokens (alphanumeric)
pub const CSRF_ALLOWED_CHARS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// CSRF session key
pub const CSRF_SESSION_KEY: &str = "_csrf_token";

// Rejection reasons
pub const REASON_BAD_ORIGIN: &str = "Origin checking failed - does not match any trusted origins.";
pub const REASON_BAD_REFERER: &str =
    "Referer checking failed - does not match any trusted origins.";
pub const REASON_CSRF_TOKEN_MISSING: &str = "CSRF token missing.";
pub const REASON_INCORRECT_LENGTH: &str = "CSRF token has incorrect length.";
pub const REASON_INSECURE_REFERER: &str =
    "Referer checking failed - Referer is insecure while host is secure.";
pub const REASON_INVALID_CHARACTERS: &str = "CSRF token has invalid characters.";
pub const REASON_MALFORMED_REFERER: &str = "Referer checking failed - Referer is malformed.";
pub const REASON_NO_CSRF_COOKIE: &str = "CSRF cookie not set.";
pub const REASON_NO_REFERER: &str = "Referer checking failed - no Referer.";

/// CSRF token validation error
#[derive(Debug)]
pub struct RejectRequest {
    pub reason: String,
}

/// Invalid token format error
#[derive(Debug)]
pub struct InvalidTokenFormat {
    pub reason: String,
}

/// CSRF metadata
#[derive(Debug, Clone)]
pub struct CsrfMeta {
    pub token: String,
}

/// SameSite cookie attribute
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    /// Strict mode - cookie only sent in first-party context
    Strict,
    /// Lax mode - cookie sent with top-level navigation
    Lax,
    /// None mode - cookie sent in all contexts (requires Secure)
    None,
}

impl Default for SameSite {
    fn default() -> Self {
        Self::Lax
    }
}

/// CSRF configuration
///
/// All tokens are generated using HMAC-SHA256 for cryptographic security.
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    pub cookie_name: String,
    pub header_name: String,
    /// CSRF cookie should NOT be HttpOnly (JavaScript needs access)
    pub cookie_httponly: bool,
    /// Cookie should be Secure in production (HTTPS only)
    pub cookie_secure: bool,
    /// SameSite attribute for CSRF protection
    pub cookie_samesite: SameSite,
    /// Cookie domain (None = current domain only)
    pub cookie_domain: Option<String>,
    /// Cookie path (default: "/")
    pub cookie_path: String,
    /// Cookie max age in seconds (None = session cookie)
    pub cookie_max_age: Option<i64>,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            cookie_name: "csrftoken".to_string(),
            header_name: "X-CSRFToken".to_string(),
            cookie_httponly: false, // CSRF token needs JavaScript access
            cookie_secure: false,   // Development default (set to true in production)
            cookie_samesite: SameSite::Lax,
            cookie_domain: None,
            cookie_path: "/".to_string(),
            cookie_max_age: None, // Session cookie
        }
    }
}

impl CsrfConfig {
    /// Production-ready configuration with security hardening
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_security::csrf::CsrfConfig;
    ///
    /// let config = CsrfConfig::production();
    /// assert!(config.cookie_secure);
    /// assert_eq!(config.cookie_path, "/");
    /// ```
    pub fn production() -> Self {
        Self {
            cookie_name: "csrftoken".to_string(),
            header_name: "X-CSRFToken".to_string(),
            cookie_httponly: false, // CSRF token needs JavaScript access
            cookie_secure: true,    // HTTPS only in production
            cookie_samesite: SameSite::Strict,
            cookie_domain: None,
            cookie_path: "/".to_string(),
            cookie_max_age: Some(31449600), // 1 year
        }
    }
}

/// CSRF middleware
pub struct CsrfMiddleware {
    #[allow(dead_code)]
    config: CsrfConfig,
}

impl CsrfMiddleware {
    pub fn new() -> Self {
        Self {
            config: CsrfConfig::default(),
        }
    }

    pub fn with_config(config: CsrfConfig) -> Self {
        Self { config }
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF token
#[derive(Debug, Clone)]
pub struct CsrfToken(pub String);

impl CsrfToken {
    pub fn new(token: String) -> Self {
        Self(token)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// HMAC-SHA256 type alias
type HmacSha256 = Hmac<Sha256>;

/// Generate HMAC-SHA256 based CSRF token
///
/// Creates a cryptographically secure token using HMAC-SHA256.
/// This is more secure than the legacy masking approach.
///
/// # Arguments
///
/// * `secret` - Secret key for HMAC (should be at least 32 bytes)
/// * `message` - Message to authenticate (typically timestamp or session ID)
///
/// # Returns
///
/// Hex-encoded HMAC token (64 characters)
///
/// # Examples
///
/// ```
/// use reinhardt_security::csrf::generate_token_hmac;
///
/// let secret = b"my-secret-key-at-least-32-bytes-long";
/// let message = "session-id-12345";
/// let token = generate_token_hmac(secret, message);
/// assert_eq!(token.len(), 64); // HMAC-SHA256 produces 32 bytes = 64 hex chars
/// ```
pub fn generate_token_hmac(secret: &[u8], message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Verify HMAC-SHA256 based CSRF token
///
/// Verifies that the token was generated with the given secret and message.
/// Uses constant-time comparison to prevent timing attacks.
///
/// # Arguments
///
/// * `token` - Hex-encoded HMAC token to verify
/// * `secret` - Secret key used for HMAC generation
/// * `message` - Original message that was authenticated
///
/// # Returns
///
/// `true` if the token is valid, `false` otherwise
///
/// # Examples
///
/// ```
/// use reinhardt_security::csrf::{generate_token_hmac, verify_token_hmac};
///
/// let secret = b"my-secret-key-at-least-32-bytes-long";
/// let message = "session-id-12345";
/// let token = generate_token_hmac(secret, message);
///
/// assert!(verify_token_hmac(&token, secret, message));
/// assert!(!verify_token_hmac(&token, secret, "different-message"));
/// assert!(!verify_token_hmac("invalid-token", secret, message));
/// ```
pub fn verify_token_hmac(token: &str, secret: &[u8], message: &str) -> bool {
    // Decode hex token
    let Ok(token_bytes) = hex::decode(token) else {
        return false;
    };

    // Generate expected HMAC
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    // Constant-time comparison to prevent timing attacks
    mac.verify_slice(&token_bytes).is_ok()
}

/// Get CSRF secret as bytes (32 bytes)
///
/// Generates a cryptographically secure random secret suitable for HMAC.
///
/// # Examples
///
/// ```
/// use reinhardt_security::csrf::get_secret_bytes;
///
/// let secret = get_secret_bytes();
/// assert_eq!(secret.len(), 32);
/// ```
pub fn get_secret_bytes() -> Vec<u8> {
    let mut rng = rand::rng();
    let mut secret = vec![0u8; 32];
    rng.fill(&mut secret[..]);
    secret
}

/// Get CSRF token using HMAC-SHA256
///
/// Generates a CSRF token using the HMAC-SHA256 approach.
/// This is the recommended method for new implementations.
///
/// # Arguments
///
/// * `secret_bytes` - 32-byte secret key
/// * `session_id` - Session identifier or timestamp
///
/// # Returns
///
/// Hex-encoded HMAC token (64 characters)
///
/// # Examples
///
/// ```
/// use reinhardt_security::csrf::{get_secret_bytes, get_token_hmac};
///
/// let secret = get_secret_bytes();
/// let session_id = "user-session-12345";
/// let token = get_token_hmac(&secret, session_id);
/// assert_eq!(token.len(), 64);
/// ```
pub fn get_token_hmac(secret_bytes: &[u8], session_id: &str) -> String {
    generate_token_hmac(secret_bytes, session_id)
}

/// Check HMAC-based CSRF token validity
///
/// Verifies a CSRF token generated with HMAC-SHA256.
///
/// # Arguments
///
/// * `request_token` - Token from the request
/// * `secret_bytes` - Secret key used for generation
/// * `session_id` - Session identifier or timestamp
///
/// # Returns
///
/// `Ok(())` if valid, `Err(RejectRequest)` if invalid
///
/// # Examples
///
/// ```
/// use reinhardt_security::csrf::{get_secret_bytes, get_token_hmac, check_token_hmac};
///
/// let secret = get_secret_bytes();
/// let session_id = "user-session-12345";
/// let token = get_token_hmac(&secret, session_id);
///
/// assert!(check_token_hmac(&token, &secret, session_id).is_ok());
/// assert!(check_token_hmac("invalid", &secret, session_id).is_err());
/// ```
pub fn check_token_hmac(
    request_token: &str,
    secret_bytes: &[u8],
    session_id: &str,
) -> Result<(), RejectRequest> {
    if !verify_token_hmac(request_token, secret_bytes, session_id) {
        return Err(RejectRequest {
            reason: "CSRF token mismatch (HMAC verification failed)".to_string(),
        });
    }
    Ok(())
}

/// Check origin header
pub fn check_origin(origin: &str, allowed_origins: &[String]) -> Result<(), RejectRequest> {
    if !allowed_origins.iter().any(|o| o == origin) {
        return Err(RejectRequest {
            reason: REASON_BAD_ORIGIN.to_string(),
        });
    }
    Ok(())
}

/// Check referer header
pub fn check_referer(
    referer: Option<&str>,
    allowed_origins: &[String],
    is_secure: bool,
) -> Result<(), RejectRequest> {
    let referer = referer.ok_or_else(|| RejectRequest {
        reason: REASON_NO_REFERER.to_string(),
    })?;

    if referer.is_empty() {
        return Err(RejectRequest {
            reason: REASON_MALFORMED_REFERER.to_string(),
        });
    }

    if is_secure && referer.starts_with("http://") {
        return Err(RejectRequest {
            reason: REASON_INSECURE_REFERER.to_string(),
        });
    }

    if !allowed_origins.iter().any(|o| referer.starts_with(o)) {
        return Err(RejectRequest {
            reason: REASON_BAD_REFERER.to_string(),
        });
    }

    Ok(())
}

/// Check if two domains are the same
pub fn is_same_domain(domain1: &str, domain2: &str) -> bool {
    domain1 == domain2
}
