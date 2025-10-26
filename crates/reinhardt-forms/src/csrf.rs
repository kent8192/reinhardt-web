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
    /// Create a new CSRF token with default settings
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
    pub fn rotate(&mut self, new_token: String) {
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

/// CSRF validator with advanced security features
#[derive(Debug, Clone)]
pub struct CsrfValidator {
    check_origin: bool,
    check_referer: bool,
    trusted_origins: Vec<String>,
}

impl CsrfValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self {
            check_origin: true,
            check_referer: true,
            trusted_origins: Vec::new(),
        }
    }

    /// Add a trusted origin
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

    /// Validate origin header
    pub fn validate_origin(&self, origin: Option<&str>, expected: &str) -> Result<(), CsrfError> {
        if !self.check_origin {
            return Ok(());
        }

        let origin = origin.ok_or(CsrfError::OriginMismatch)?;

        if origin == expected || self.trusted_origins.iter().any(|t| t == origin) {
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
        let mut token = CsrfToken::with_rotation("old-token".to_string());
        assert!(token.rotation_enabled());

        token.rotate("new-token".to_string());
        assert_eq!(token.token(), "new-token");
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
