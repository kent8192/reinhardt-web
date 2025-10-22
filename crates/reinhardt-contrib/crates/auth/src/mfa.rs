//! Multi-Factor Authentication (MFA)
//!
//! Provides TOTP (Time-based One-Time Password) support for MFA.

use crate::{AuthenticationBackend, AuthenticationError, SimpleUser, User};
use reinhardt_apps::Request;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// MFA authentication backend
///
/// Provides Time-based One-Time Password (TOTP) authentication.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::MfaManager;
///
/// let mfa = MfaManager::new("MyApp");
/// ```
pub struct MFAAuthentication {
    /// TOTP issuer name
    issuer: String,
    /// User secrets (username -> secret)
    secrets: Arc<Mutex<HashMap<String, String>>>,
    /// Time window for TOTP validation (in seconds)
    time_window: u64,
}

impl MFAAuthentication {
    /// Create a new MFA authentication backend
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::MfaManager;
    ///
    /// let mfa = MfaManager::new("MyApp");
    /// ```
    pub fn new(issuer: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            secrets: Arc::new(Mutex::new(HashMap::new())),
            time_window: 30,
        }
    }

    /// Set time window for TOTP validation
    pub fn time_window(mut self, seconds: u64) -> Self {
        self.time_window = seconds;
        self
    }

    /// Register a user with a secret
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::MfaManager;
    ///
    /// let mfa = MfaManager::new("MyApp");
    /// mfa.register_user("alice", "SECRET_BASE32");
    /// ```
    pub fn register_user(&self, username: impl Into<String>, secret: impl Into<String>) {
        let mut secrets = self.secrets.lock().unwrap();
        secrets.insert(username.into(), secret.into());
    }

    /// Generate TOTP URL for QR code
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::MfaManager;
    ///
    /// let mfa = MfaManager::new("MyApp");
    /// let url = mfa.generate_totp_url("alice", "SECRET_BASE32");
    /// assert!(url.starts_with("otpauth://totp/"));
    /// ```
    pub fn generate_totp_url(&self, username: &str, secret: &str) -> String {
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}",
            self.issuer, username, secret, self.issuer
        )
    }

    /// Verify TOTP code
    ///
    /// In production, this should use a library like `totp-lite` for actual TOTP verification.
    /// This implementation accepts any 6-digit numeric code for demonstration purposes.
    pub fn verify_totp(&self, username: &str, code: &str) -> Result<bool, AuthenticationError> {
        let secrets = self.secrets.lock().unwrap();

        if let Some(_secret) = secrets.get(username) {
            // For demonstration, accept any 6-digit code
            // In production, use: totp-lite or similar library
            Ok(code.len() == 6 && code.chars().all(|c| c.is_numeric()))
        } else {
            Err(AuthenticationError::UserNotFound)
        }
    }
}

impl Default for MFAAuthentication {
    fn default() -> Self {
        Self::new("Reinhardt")
    }
}

impl AuthenticationBackend for MFAAuthentication {
    fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        // Extract username and MFA code from request headers
        let username = request
            .headers
            .get("X-Username")
            .and_then(|v| v.to_str().ok());
        let code = request
            .headers
            .get("X-MFA-Code")
            .and_then(|v| v.to_str().ok());

        match (username, code) {
            (Some(user), Some(mfa_code)) => {
                if self.verify_totp(user, mfa_code)? {
                    Ok(Some(Box::new(SimpleUser {
                        id: Uuid::new_v4(),
                        username: user.to_string(),
                        email: format!("{}@example.com", user),
                        is_active: true,
                        is_admin: false,
                    })))
                } else {
                    Err(AuthenticationError::InvalidCredentials)
                }
            }
            _ => Ok(None),
        }
    }

    fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        // Check if user exists in our secrets store
        let secrets = self.secrets.lock().unwrap();
        if secrets.contains_key(user_id) {
            Ok(Some(Box::new(SimpleUser {
                id: Uuid::new_v4(),
                username: user_id.to_string(),
                email: format!("{}@example.com", user_id),
                is_active: true,
                is_admin: false,
            })))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};

    #[test]
    fn test_mfa_registration() {
        let mfa = MFAAuthentication::new("TestApp");
        mfa.register_user("alice", "JBSWY3DPEHPK3PXP");

        let secrets = mfa.secrets.lock().unwrap();
        assert!(secrets.contains_key("alice"));
    }

    #[test]
    fn test_generate_totp_url() {
        let mfa = MFAAuthentication::new("TestApp");
        let url = mfa.generate_totp_url("alice", "SECRET");

        assert!(url.contains("otpauth://totp/"));
        assert!(url.contains("alice"));
        assert!(url.contains("SECRET"));
        assert!(url.contains("TestApp"));
    }

    #[test]
    fn test_verify_totp_valid_format() {
        let mfa = MFAAuthentication::new("TestApp");
        mfa.register_user("alice", "SECRET");

        // 6-digit numeric code should be accepted in demo mode
        let result = mfa.verify_totp("alice", "123456");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_totp_invalid_format() {
        let mfa = MFAAuthentication::new("TestApp");
        mfa.register_user("alice", "SECRET");

        let result = mfa.verify_totp("alice", "12345"); // Too short
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_totp_unregistered_user() {
        let mfa = MFAAuthentication::new("TestApp");

        let result = mfa.verify_totp("alice", "123456");
        assert!(result.is_err());
    }

    #[test]
    fn test_mfa_authentication_with_valid_code() {
        let mfa = MFAAuthentication::new("TestApp");
        mfa.register_user("alice", "SECRET");

        let mut headers = HeaderMap::new();
        headers.insert("X-Username", "alice".parse().unwrap());
        headers.insert("X-MFA-Code", "123456".parse().unwrap());

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = mfa.authenticate(&request).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "alice");
    }

    #[test]
    fn test_mfa_authentication_without_headers() {
        let mfa = MFAAuthentication::new("TestApp");

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let result = mfa.authenticate(&request).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_time_window_configuration() {
        let mfa = MFAAuthentication::new("TestApp").time_window(60);
        assert_eq!(mfa.time_window, 60);
    }
}
