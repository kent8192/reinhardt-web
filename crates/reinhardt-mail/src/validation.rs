//! Email validation and sanitization utilities

use crate::{EmailError, EmailResult};
use idna::domain_to_ascii;

/// Validates an email address according to RFC 5321/5322 standards
///
/// # Examples
///
/// ```
/// use reinhardt_mail::validation::validate_email;
///
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("user+tag@example.com").is_ok());
/// assert!(validate_email("invalid@").is_err());
/// assert!(validate_email("@example.com").is_err());
/// ```
pub fn validate_email(email: &str) -> EmailResult<()> {
	if email.is_empty() {
		return Err(EmailError::InvalidAddress(
			"Email cannot be empty".to_string(),
		));
	}

	// Check for header injection attempts
	if email.contains('\n') || email.contains('\r') {
		return Err(EmailError::HeaderInjection(
			"Email address contains newline characters".to_string(),
		));
	}

	// Count @ symbols
	let at_count = email.chars().filter(|&c| c == '@').count();
	if at_count != 1 {
		return Err(EmailError::InvalidAddress(format!(
			"Email must contain exactly one @ symbol, found {}",
			at_count
		)));
	}

	// Split into local and domain parts
	let parts: Vec<&str> = email.split('@').collect();
	let local = parts[0];
	let domain = parts[1];

	// Validate local part
	validate_local_part(local)?;

	// Validate domain part
	validate_domain(domain)?;

	Ok(())
}

/// Validates the local part of an email address (before @)
fn validate_local_part(local: &str) -> EmailResult<()> {
	if local.is_empty() {
		return Err(EmailError::InvalidAddress(
			"Local part cannot be empty".to_string(),
		));
	}

	if local.len() > 64 {
		return Err(EmailError::InvalidAddress(
			"Local part is too long (max 64 characters)".to_string(),
		));
	}

	// Check for invalid characters
	if local.starts_with('.') || local.ends_with('.') {
		return Err(EmailError::InvalidAddress(
			"Local part cannot start or end with a dot".to_string(),
		));
	}

	if local.contains("..") {
		return Err(EmailError::InvalidAddress(
			"Local part cannot contain consecutive dots".to_string(),
		));
	}

	Ok(())
}

/// Validates the domain part of an email address (after @)
fn validate_domain(domain: &str) -> EmailResult<()> {
	if domain.is_empty() {
		return Err(EmailError::InvalidAddress(
			"Domain cannot be empty".to_string(),
		));
	}

	if domain.len() > 253 {
		return Err(EmailError::InvalidAddress(
			"Domain is too long (max 253 characters)".to_string(),
		));
	}

	// Check for invalid characters
	if domain.starts_with('.') || domain.ends_with('.') {
		return Err(EmailError::InvalidAddress(
			"Domain cannot start or end with a dot".to_string(),
		));
	}

	if domain.starts_with('-') || domain.ends_with('-') {
		return Err(EmailError::InvalidAddress(
			"Domain cannot start or end with a hyphen".to_string(),
		));
	}

	// Try to convert to ASCII using IDNA (for international domains)
	if domain_to_ascii(domain).is_err() {
		return Err(EmailError::InvalidAddress(
			"Invalid domain name".to_string(),
		));
	}

	Ok(())
}

/// Validates a list of email addresses
///
/// # Examples
///
/// ```
/// use reinhardt_mail::validation::validate_email_list;
///
/// let emails = vec!["user1@example.com", "user2@example.com"];
/// assert!(validate_email_list(&emails).is_ok());
///
/// let invalid_emails = vec!["valid@example.com", "invalid@"];
/// assert!(validate_email_list(&invalid_emails).is_err());
/// ```
pub fn validate_email_list(emails: &[impl AsRef<str>]) -> EmailResult<()> {
	for email in emails {
		validate_email(email.as_ref())?;
	}
	Ok(())
}

/// Sanitizes an email address by trimming whitespace and lowercasing the domain.
///
/// Per RFC 5321 Section 2.4, the local part of an email address is case-sensitive,
/// while the domain part is case-insensitive. This function only lowercases the
/// domain part, preserving the original case of the local part.
///
/// # Examples
///
/// ```
/// use reinhardt_mail::validation::sanitize_email;
///
/// assert_eq!(sanitize_email("  User@Example.COM  ").unwrap(), "User@example.com");
/// assert_eq!(sanitize_email("user+tag@EXAMPLE.com").unwrap(), "user+tag@example.com");
/// ```
pub fn sanitize_email(email: &str) -> EmailResult<String> {
	let trimmed = email.trim();
	validate_email(trimmed)?;

	// RFC 5321: local part is case-sensitive, domain is case-insensitive
	let (local, domain) = trimmed.rsplit_once('@').ok_or_else(|| {
		EmailError::InvalidAddress("Email must contain exactly one @ symbol, found 0".to_string())
	})?;
	Ok(format!("{}@{}", local, domain.to_lowercase()))
}

/// Sanitizes a list of email addresses
///
/// Preserves case of local parts and lowercases domain parts per RFC 5321.
///
/// # Examples
///
/// ```
/// use reinhardt_mail::validation::sanitize_email_list;
///
/// let emails = vec!["  User1@Example.COM  ", "User2@EXAMPLE.com"];
/// let sanitized = sanitize_email_list(&emails).unwrap();
/// assert_eq!(sanitized, vec!["User1@example.com", "User2@example.com"]);
/// ```
pub fn sanitize_email_list(emails: &[impl AsRef<str>]) -> EmailResult<Vec<String>> {
	emails.iter().map(|e| sanitize_email(e.as_ref())).collect()
}

/// Checks if a string contains potential header injection attempts
///
/// # Examples
///
/// ```
/// use reinhardt_mail::validation::check_header_injection;
///
/// assert!(check_header_injection("Normal subject").is_ok());
/// assert!(check_header_injection("Subject\nBcc: attacker@evil.com").is_err());
/// assert!(check_header_injection("Subject\rCc: attacker@evil.com").is_err());
/// ```
pub fn check_header_injection(value: &str) -> EmailResult<()> {
	if value.contains('\n') || value.contains('\r') {
		return Err(EmailError::HeaderInjection(
			"Value contains newline characters".to_string(),
		));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_validate_email_valid() {
		assert!(validate_email("user@example.com").is_ok());
		assert!(validate_email("user+tag@example.com").is_ok());
		assert!(validate_email("user.name@example.com").is_ok());
		assert!(validate_email("user_name@example.com").is_ok());
		assert!(validate_email("user123@example.co.uk").is_ok());
	}

	#[test]
	fn test_validate_email_invalid() {
		assert!(validate_email("").is_err());
		assert!(validate_email("invalid").is_err());
		assert!(validate_email("@example.com").is_err());
		assert!(validate_email("user@").is_err());
		assert!(validate_email("user@@example.com").is_err());
		assert!(validate_email("user@example@com").is_err());
	}

	#[test]
	fn test_validate_email_header_injection() {
		assert!(validate_email("user@example.com\nBcc: attacker@evil.com").is_err());
		assert!(validate_email("user@example.com\rCc: attacker@evil.com").is_err());
	}

	#[test]
	fn test_validate_local_part() {
		assert!(validate_local_part("user").is_ok());
		assert!(validate_local_part("user.name").is_ok());
		assert!(validate_local_part("user+tag").is_ok());

		assert!(validate_local_part("").is_err());
		assert!(validate_local_part(".user").is_err());
		assert!(validate_local_part("user.").is_err());
		assert!(validate_local_part("user..name").is_err());
	}

	#[test]
	fn test_validate_domain() {
		assert!(validate_domain("example.com").is_ok());
		assert!(validate_domain("mail.example.com").is_ok());
		assert!(validate_domain("example.co.uk").is_ok());

		assert!(validate_domain("").is_err());
		assert!(validate_domain(".example.com").is_err());
		assert!(validate_domain("example.com.").is_err());
		assert!(validate_domain("-example.com").is_err());
		assert!(validate_domain("example.com-").is_err());
	}

	#[test]
	fn test_sanitize_email() {
		// RFC 5321: local part is case-sensitive, only domain is lowercased
		assert_eq!(
			sanitize_email("  User@Example.COM  ").unwrap(),
			"User@example.com"
		);
		assert_eq!(
			sanitize_email("USER+TAG@EXAMPLE.COM").unwrap(),
			"USER+TAG@example.com"
		);
		assert_eq!(
			sanitize_email("john.smith@example.com").unwrap(),
			"john.smith@example.com"
		);
	}

	#[test]
	fn test_sanitize_email_list() {
		let emails = vec!["  User1@Example.COM  ", "User2@EXAMPLE.com"];
		let sanitized = sanitize_email_list(&emails).unwrap();
		assert_eq!(sanitized, vec!["User1@example.com", "User2@example.com"]);
	}

	#[test]
	fn test_check_header_injection() {
		assert!(check_header_injection("Normal subject").is_ok());
		assert!(check_header_injection("Subject with spaces").is_ok());

		assert!(check_header_injection("Subject\nBcc: attacker@evil.com").is_err());
		assert!(check_header_injection("Subject\rCc: attacker@evil.com").is_err());
		assert!(check_header_injection("Subject\r\nTo: attacker@evil.com").is_err());
	}

	#[test]
	fn test_validate_email_list() {
		let valid_emails = vec!["user1@example.com", "user2@example.com"];
		assert!(validate_email_list(&valid_emails).is_ok());

		let invalid_emails = vec!["valid@example.com", "invalid@"];
		assert!(validate_email_list(&invalid_emails).is_err());
	}
}
