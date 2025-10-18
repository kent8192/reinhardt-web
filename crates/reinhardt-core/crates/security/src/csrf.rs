//! CSRF (Cross-Site Request Forgery) protection

use rand::Rng;

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

/// Mask a CSRF secret to create a token
pub fn mask_cipher_secret(secret: &str) -> String {
    let mut rng = rand::rng();
    let mask: String = (0..CSRF_SECRET_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CSRF_ALLOWED_CHARS.len());
            CSRF_ALLOWED_CHARS.chars().nth(idx).unwrap()
        })
        .collect();

    let masked: String = secret
        .chars()
        .zip(mask.chars())
        .map(|(s, m)| {
            let s_idx = CSRF_ALLOWED_CHARS.find(s).unwrap();
            let m_idx = CSRF_ALLOWED_CHARS.find(m).unwrap();
            let result_idx = (s_idx + m_idx) % CSRF_ALLOWED_CHARS.len();
            CSRF_ALLOWED_CHARS.chars().nth(result_idx).unwrap()
        })
        .collect();

    format!("{}{}", mask, masked)
}

/// Unmask a CSRF token to get the secret
pub fn unmask_cipher_token(token: &str) -> String {
    let (mask, masked) = token.split_at(CSRF_SECRET_LENGTH);

    mask.chars()
        .zip(masked.chars())
        .map(|(m, masked_char)| {
            let m_idx = CSRF_ALLOWED_CHARS.find(m).unwrap();
            let masked_idx = CSRF_ALLOWED_CHARS.find(masked_char).unwrap();
            let secret_idx =
                (masked_idx + CSRF_ALLOWED_CHARS.len() - m_idx) % CSRF_ALLOWED_CHARS.len();
            CSRF_ALLOWED_CHARS.chars().nth(secret_idx).unwrap()
        })
        .collect()
}

/// Check token format
pub fn check_token_format(token: &str) -> Result<(), InvalidTokenFormat> {
    if token.len() != CSRF_TOKEN_LENGTH {
        return Err(InvalidTokenFormat {
            reason: REASON_INCORRECT_LENGTH.to_string(),
        });
    }

    if !token.chars().all(|c| CSRF_ALLOWED_CHARS.contains(c)) {
        return Err(InvalidTokenFormat {
            reason: REASON_INVALID_CHARACTERS.to_string(),
        });
    }

    Ok(())
}

/// Check if two tokens match
pub fn does_token_match(token1: &str, token2: &str) -> bool {
    token1 == token2
}

/// Check token validity
pub fn check_token(request_token: &str, secret: &str) -> Result<(), RejectRequest> {
    check_token_format(request_token)?;
    let request_secret = unmask_cipher_token(request_token);

    if !does_token_match(&request_secret, secret) {
        return Err(RejectRequest {
            reason: "CSRF token mismatch".to_string(),
        });
    }

    Ok(())
}

impl From<InvalidTokenFormat> for RejectRequest {
    fn from(err: InvalidTokenFormat) -> Self {
        RejectRequest { reason: err.reason }
    }
}

/// Get CSRF secret from session
pub fn get_secret() -> String {
    let mut rng = rand::rng();
    (0..CSRF_SECRET_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CSRF_ALLOWED_CHARS.len());
            CSRF_ALLOWED_CHARS.chars().nth(idx).unwrap()
        })
        .collect()
}

/// Get CSRF token
pub fn get_token(secret: &str) -> String {
    mask_cipher_secret(secret)
}

/// Rotate CSRF token
pub fn rotate_token() -> String {
    let secret = get_secret();
    mask_cipher_secret(&secret)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_unmask() {
        let secret = "abcdefghijklmnopqrstuvwxyz012345";
        let masked = mask_cipher_secret(secret);
        let unmasked = unmask_cipher_token(&masked);
        assert_eq!(unmasked, secret);
    }

    #[test]
    fn test_token_format_valid() {
        let token = "a".repeat(CSRF_TOKEN_LENGTH);
        assert!(check_token_format(&token).is_ok());
    }

    #[test]
    fn test_token_format_invalid_length() {
        let token = "a".repeat(CSRF_TOKEN_LENGTH - 1);
        assert!(check_token_format(&token).is_err());
    }

    #[test]
    fn test_does_token_match() {
        assert!(does_token_match("abc", "abc"));
        assert!(!does_token_match("abc", "abd"));
    }
}
