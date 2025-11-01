//! Form security functionality
//!
//! This module provides security features for forms including CSRF protection,
//! honeypot fields for bot detection, and rate limiting.
//!
//! ## CSRF Protection
//!
//! For advanced CSRF protection including origin validation, SameSite cookies,
//! and cryptographic token generation, see the [`csrf`](crate::csrf) module,
//! specifically [`CsrfValidator`](crate::csrf::CsrfValidator).

use crate::Form;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
	#[error("CSRF validation failed: {0}")]
	CsrfValidationFailed(String),
	#[error("Rate limit exceeded: {0}")]
	RateLimitExceeded(String),
	#[error("Bot detected: {0}")]
	BotDetected(String),
}

/// FormSecurityMiddleware provides comprehensive form security
///
/// Includes CSRF protection, honeypot fields, and optional rate limiting.
pub struct FormSecurityMiddleware {
	csrf_enabled: bool,
	honeypot_field: Option<String>,
	csrf_secret: Option<String>,
}

impl FormSecurityMiddleware {
	/// Create a new FormSecurityMiddleware
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormSecurityMiddleware;
	///
	/// let middleware = FormSecurityMiddleware::new();
	/// assert!(!middleware.csrf_enabled());
	/// ```
	pub fn new() -> Self {
		Self {
			csrf_enabled: false,
			honeypot_field: None,
			csrf_secret: None,
		}
	}

	/// Enable CSRF protection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormSecurityMiddleware;
	///
	/// let middleware = FormSecurityMiddleware::new()
	///     .with_csrf(Some("secret-key".to_string()));
	/// assert!(middleware.csrf_enabled());
	/// ```
	pub fn with_csrf(mut self, secret: Option<String>) -> Self {
		self.csrf_enabled = true;
		self.csrf_secret = secret;
		self
	}

	/// Add a honeypot field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormSecurityMiddleware;
	///
	/// let middleware = FormSecurityMiddleware::new()
	///     .with_honeypot("email_confirm".to_string());
	/// assert!(middleware.has_honeypot());
	/// ```
	pub fn with_honeypot(mut self, field_name: String) -> Self {
		self.honeypot_field = Some(field_name);
		self
	}

	/// Check if CSRF is enabled
	pub fn csrf_enabled(&self) -> bool {
		self.csrf_enabled
	}

	/// Check if honeypot is configured
	pub fn has_honeypot(&self) -> bool {
		self.honeypot_field.is_some()
	}

	/// Get the honeypot field name
	pub fn honeypot_field(&self) -> Option<&str> {
		self.honeypot_field.as_deref()
	}

	/// Validate CSRF token
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormSecurityMiddleware;
	///
	/// let middleware = FormSecurityMiddleware::new()
	///     .with_csrf(Some("secret".to_string()));
	///
	/// // Valid token validation would require actual CSRF token generation
	/// // This is a simplified example
	/// ```
	pub fn validate_csrf(&self, token: &str) -> Result<(), SecurityError> {
		if !self.csrf_enabled {
			return Ok(());
		}

		if token.is_empty() {
			return Err(SecurityError::CsrfValidationFailed(
				"CSRF token is empty".to_string(),
			));
		}

		// In a real implementation, this would verify the token signature
		// For now, we just check it's not empty
		Ok(())
	}

	/// Validate honeypot field
	///
	/// The honeypot field should be empty in legitimate submissions.
	/// If it contains any value, it's likely a bot.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormSecurityMiddleware;
	/// use std::collections::HashMap;
	///
	/// let middleware = FormSecurityMiddleware::new()
	///     .with_honeypot("email_confirm".to_string());
	///
	/// let mut data = HashMap::new();
	/// data.insert("email_confirm".to_string(), serde_json::json!(""));
	///
	/// let result = middleware.validate_honeypot(&data);
	/// assert!(result.is_ok());
	/// ```
	pub fn validate_honeypot(
		&self,
		data: &HashMap<String, serde_json::Value>,
	) -> Result<(), SecurityError> {
		if let Some(ref field) = self.honeypot_field
			&& let Some(value) = data.get(field) {
				// Honeypot should be empty
				if !value.is_null()
					&& !(value.is_string() && value.as_str().unwrap_or("").is_empty())
				{
					return Err(SecurityError::BotDetected(
						"Honeypot field was filled".to_string(),
					));
				}
			}
		Ok(())
	}

	/// Apply security checks to a form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormSecurityMiddleware, Form};
	/// use std::collections::HashMap;
	///
	/// let middleware = FormSecurityMiddleware::new()
	///     .with_csrf(Some("secret".to_string()))
	///     .with_honeypot("bot_trap".to_string());
	///
	/// let mut form = Form::new();
	/// let mut data = HashMap::new();
	/// data.insert("bot_trap".to_string(), serde_json::json!(""));
	///
	/// // In real usage, you would also include CSRF token in data
	/// ```
	pub fn apply_to_form(&self, form: &mut Form) {
		if self.csrf_enabled {
			form.enable_csrf(self.csrf_secret.clone());
		}
	}
}

impl Default for FormSecurityMiddleware {
	fn default() -> Self {
		Self::new()
	}
}

/// HoneypotField is a hidden field used to detect bots
///
/// Legitimate users won't see or fill this field, but bots often
/// auto-fill all form fields.
pub struct HoneypotField {
	name: String,
	label: Option<String>,
}

impl HoneypotField {
	/// Create a new honeypot field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("email_confirm".to_string());
	/// assert_eq!(honeypot.name(), "email_confirm");
	/// ```
	pub fn new(name: String) -> Self {
		Self { name, label: None }
	}

	/// Set the field label
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("trap".to_string())
	///     .with_label("Please leave this field empty".to_string());
	/// assert_eq!(honeypot.label(), Some("Please leave this field empty"));
	/// ```
	pub fn with_label(mut self, label: String) -> Self {
		self.label = Some(label);
		self
	}

	/// Get the field name
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get the field label
	pub fn label(&self) -> Option<&str> {
		self.label.as_deref()
	}

	/// Render the honeypot field as HTML
	///
	/// The field is hidden with CSS to prevent legitimate users from seeing it.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("email_confirm".to_string());
	/// let html = honeypot.render();
	/// assert!(html.contains("style=\"display:none\""));
	/// ```
	pub fn render(&self) -> String {
		format!(
			r#"<div style="display:none"><label for="{}">{}</label><input type="text" name="{}" id="{}" tabindex="-1" autocomplete="off" /></div>"#,
			self.name,
			self.label.as_deref().unwrap_or(""),
			self.name,
			self.name
		)
	}

	/// Validate the honeypot field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("trap".to_string());
	///
	/// // Empty value is valid (not a bot)
	/// assert!(honeypot.validate(None).is_ok());
	/// assert!(honeypot.validate(Some("")).is_ok());
	///
	/// // Non-empty value indicates bot
	/// assert!(honeypot.validate(Some("bot-filled-this")).is_err());
	/// ```
	pub fn validate(&self, value: Option<&str>) -> Result<(), SecurityError> {
		match value {
			None | Some("") => Ok(()),
			Some(_) => Err(SecurityError::BotDetected(format!(
				"Honeypot field '{}' was filled",
				self.name
			))),
		}
	}
}

/// RateLimiter prevents abuse by limiting form submissions
///
/// Tracks submission attempts and enforces rate limits per identifier
/// (e.g., IP address or user ID).
pub struct RateLimiter {
	requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
	max_requests: usize,
	window: Duration,
}

impl RateLimiter {
	/// Create a new RateLimiter
	///
	/// # Arguments
	///
	/// * `max_requests` - Maximum number of requests allowed in the time window
	/// * `window` - Time window duration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::RateLimiter;
	/// use std::time::Duration;
	///
	/// let limiter = RateLimiter::new(5, Duration::from_secs(60));
	/// assert_eq!(limiter.max_requests(), 5);
	/// ```
	pub fn new(max_requests: usize, window: Duration) -> Self {
		Self {
			requests: Arc::new(Mutex::new(HashMap::new())),
			max_requests,
			window,
		}
	}

	/// Get maximum requests allowed
	pub fn max_requests(&self) -> usize {
		self.max_requests
	}

	/// Get the time window
	pub fn window(&self) -> Duration {
		self.window
	}

	/// Check if a request is allowed
	///
	/// # Arguments
	///
	/// * `identifier` - Unique identifier (e.g., IP address, user ID)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::RateLimiter;
	/// use std::time::Duration;
	///
	/// let limiter = RateLimiter::new(3, Duration::from_secs(10));
	///
	/// // First 3 requests should be allowed
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	///
	/// // 4th request should be rate limited
	/// assert!(limiter.check_rate_limit("user1").is_err());
	/// ```
	pub fn check_rate_limit(&self, identifier: &str) -> Result<(), SecurityError> {
		let now = Instant::now();
		let mut requests = self.requests.lock().unwrap();

		// Get or create request history for this identifier
		let history = requests
			.entry(identifier.to_string())
			.or_default();

		// Remove old requests outside the window
		history.retain(|&time| now.duration_since(time) < self.window);

		// Check if limit is exceeded
		if history.len() >= self.max_requests {
			return Err(SecurityError::RateLimitExceeded(format!(
				"Rate limit exceeded: {} requests in {:?}",
				self.max_requests, self.window
			)));
		}

		// Record this request
		history.push(now);

		Ok(())
	}

	/// Reset rate limit for an identifier
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::RateLimiter;
	/// use std::time::Duration;
	///
	/// let limiter = RateLimiter::new(2, Duration::from_secs(10));
	///
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// assert!(limiter.check_rate_limit("user1").is_err());
	///
	/// limiter.reset("user1");
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// ```
	pub fn reset(&self, identifier: &str) {
		let mut requests = self.requests.lock().unwrap();
		requests.remove(identifier);
	}

	/// Clear all rate limit data
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::RateLimiter;
	/// use std::time::Duration;
	///
	/// let limiter = RateLimiter::new(1, Duration::from_secs(10));
	///
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// assert!(limiter.check_rate_limit("user1").is_err());
	///
	/// limiter.clear_all();
	/// assert!(limiter.check_rate_limit("user1").is_ok());
	/// ```
	pub fn clear_all(&self) {
		let mut requests = self.requests.lock().unwrap();
		requests.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_form_security_middleware_creation() {
		let middleware = FormSecurityMiddleware::new();

		assert!(!middleware.csrf_enabled());
		assert!(!middleware.has_honeypot());
	}

	#[test]
	fn test_form_security_middleware_with_csrf() {
		let middleware = FormSecurityMiddleware::new().with_csrf(Some("secret-key".to_string()));

		assert!(middleware.csrf_enabled());
	}

	#[test]
	fn test_form_security_middleware_with_honeypot() {
		let middleware = FormSecurityMiddleware::new().with_honeypot("email_confirm".to_string());

		assert!(middleware.has_honeypot());
		assert_eq!(middleware.honeypot_field(), Some("email_confirm"));
	}

	#[test]
	fn test_csrf_validation() {
		let middleware = FormSecurityMiddleware::new().with_csrf(Some("secret-key".to_string()));

		assert!(middleware.validate_csrf("valid-token").is_ok());
		assert!(middleware.validate_csrf("").is_err());
	}

	#[test]
	fn test_honeypot_validation() {
		let middleware = FormSecurityMiddleware::new().with_honeypot("email_confirm".to_string());

		let mut data = HashMap::new();
		data.insert("email_confirm".to_string(), serde_json::json!(""));

		assert!(middleware.validate_honeypot(&data).is_ok());

		data.insert("email_confirm".to_string(), serde_json::json!("bot-value"));
		assert!(middleware.validate_honeypot(&data).is_err());
	}

	#[test]
	fn test_honeypot_field_creation() {
		let honeypot = HoneypotField::new("trap".to_string());

		assert_eq!(honeypot.name(), "trap");
		assert_eq!(honeypot.label(), None);
	}

	#[test]
	fn test_honeypot_field_with_label() {
		let honeypot =
			HoneypotField::new("trap".to_string()).with_label("Leave this empty".to_string());

		assert_eq!(honeypot.label(), Some("Leave this empty"));
	}

	#[test]
	fn test_honeypot_field_render() {
		let honeypot = HoneypotField::new("email_confirm".to_string());
		let html = honeypot.render();

		assert!(html.contains("style=\"display:none\""));
		assert!(html.contains("name=\"email_confirm\""));
		assert!(html.contains("tabindex=\"-1\""));
		assert!(html.contains("autocomplete=\"off\""));
	}

	#[test]
	fn test_honeypot_field_validate() {
		let honeypot = HoneypotField::new("trap".to_string());

		assert!(honeypot.validate(None).is_ok());
		assert!(honeypot.validate(Some("")).is_ok());
		assert!(honeypot.validate(Some("bot-value")).is_err());
	}

	#[test]
	fn test_rate_limiter_creation() {
		let limiter = RateLimiter::new(5, Duration::from_secs(60));

		assert_eq!(limiter.max_requests(), 5);
		assert_eq!(limiter.window(), Duration::from_secs(60));
	}

	#[test]
	fn test_rate_limiter_allows_requests_within_limit() {
		let limiter = RateLimiter::new(3, Duration::from_secs(10));

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_ok());
	}

	#[test]
	fn test_rate_limiter_blocks_excess_requests() {
		let limiter = RateLimiter::new(2, Duration::from_secs(10));

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_err());
	}

	#[test]
	fn test_rate_limiter_separate_identifiers() {
		let limiter = RateLimiter::new(2, Duration::from_secs(10));

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_err());

		// Different user should have separate limit
		assert!(limiter.check_rate_limit("user2").is_ok());
		assert!(limiter.check_rate_limit("user2").is_ok());
	}

	#[test]
	fn test_rate_limiter_reset() {
		let limiter = RateLimiter::new(1, Duration::from_secs(10));

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user1").is_err());

		limiter.reset("user1");

		assert!(limiter.check_rate_limit("user1").is_ok());
	}

	#[test]
	fn test_rate_limiter_clear_all() {
		let limiter = RateLimiter::new(1, Duration::from_secs(10));

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user2").is_ok());

		assert!(limiter.check_rate_limit("user1").is_err());
		assert!(limiter.check_rate_limit("user2").is_err());

		limiter.clear_all();

		assert!(limiter.check_rate_limit("user1").is_ok());
		assert!(limiter.check_rate_limit("user2").is_ok());
	}
}
