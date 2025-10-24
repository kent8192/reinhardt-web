//! # Reinhardt Security
//!
//! Security features for Reinhardt framework including CSRF protection,
//! XSS prevention, and security header management.
//!
//! ## Features
//!
//! - CSRF (Cross-Site Request Forgery) protection
//! - XSS (Cross-Site Scripting) prevention helpers
//! - Security headers middleware
//! - Content Security Policy (CSP)
//! - Clickjacking protection
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_security::{CsrfMiddleware, SecurityHeadersMiddleware};
//!
//! // Add CSRF protection
//! app.add_middleware(CsrfMiddleware::new());
//!
//! // Add security headers
//! app.add_middleware(SecurityHeadersMiddleware::default());
//! ```

pub mod csrf;
pub mod headers;
pub mod hsts;
pub mod utils;
pub mod xss;

pub use csrf::{CsrfConfig, CsrfMiddleware, CsrfToken};
pub use headers::{ContentSecurityPolicy, SecurityHeadersConfig, SecurityHeadersMiddleware};
pub use hsts::HstsConfig;
pub use xss::{escape_html, sanitize_html};

use thiserror::Error;

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Security-related errors
#[derive(Debug, Error)]
pub enum SecurityError {
    /// CSRF token validation failed
    #[error("CSRF validation failed: {0}")]
    CsrfValidationFailed(String),

    /// Missing CSRF token
    #[error("Missing CSRF token")]
    MissingCsrfToken,

    /// Invalid security configuration
    #[error("Invalid security configuration: {0}")]
    InvalidConfiguration(String),

    /// XSS attempt detected
    #[error("Potential XSS detected: {0}")]
    XssDetected(String),
}

// Re-export CSRF functions and types
pub use csrf::{
    mask_cipher_secret, unmask_cipher_token, InvalidTokenFormat, RejectRequest, CSRF_ALLOWED_CHARS,
    CSRF_SECRET_LENGTH, CSRF_SESSION_KEY, CSRF_TOKEN_LENGTH, REASON_BAD_ORIGIN, REASON_BAD_REFERER,
    REASON_CSRF_TOKEN_MISSING, REASON_INCORRECT_LENGTH, REASON_INSECURE_REFERER,
    REASON_INVALID_CHARACTERS, REASON_MALFORMED_REFERER, REASON_NO_CSRF_COOKIE, REASON_NO_REFERER,
};

// Re-export additional CSRF functions and types
pub use csrf::{
    check_origin, check_referer, check_token, check_token_format, does_token_match, get_secret,
    get_token, is_same_domain, rotate_token, CsrfMeta,
};
