//! IP Address validator using std::net::IpAddr for type-safe validation

use super::{ValidationError, ValidationResult, Validator};
use std::net::IpAddr;

/// IP Address validator - validates IPv4 and IPv6 addresses using std::net::IpAddr
///
/// This validator provides type-safe IP address validation by leveraging Rust's
/// standard library. It supports both IPv4 and IPv6 addresses with configurable
/// validation rules and custom error messages.
///
/// # Examples
///
/// ## Basic usage (accepts both IPv4 and IPv6)
///
/// ```
/// use reinhardt_core::validators::{IPAddressValidator, Validator};
///
/// let validator = IPAddressValidator::new();
/// assert!(validator.validate("192.168.1.1").is_ok());
/// assert!(validator.validate("2001:0db8:85a3:0000:0000:8a2e:0370:7334").is_ok());
/// assert!(validator.validate("::1").is_ok());
/// assert!(validator.validate("invalid-ip").is_err());
/// ```
///
/// ## IPv4 only
///
/// ```
/// use reinhardt_core::validators::{IPAddressValidator, Validator};
///
/// let validator = IPAddressValidator::ipv4_only();
/// assert!(validator.validate("192.168.1.1").is_ok());
/// assert!(validator.validate("2001:0db8:85a3::8a2e:0370:7334").is_err());
/// ```
///
/// ## IPv6 only
///
/// ```
/// use reinhardt_core::validators::{IPAddressValidator, Validator};
///
/// let validator = IPAddressValidator::ipv6_only();
/// assert!(validator.validate("2001:0db8:85a3::8a2e:0370:7334").is_ok());
/// assert!(validator.validate("192.168.1.1").is_err());
/// ```
///
/// ## Custom error message
///
/// ```
/// use reinhardt_core::validators::{IPAddressValidator, Validator};
///
/// let validator = IPAddressValidator::new()
///     .with_message("Please provide a valid IP address");
///
/// match validator.validate("invalid") {
///     Err(e) => {
///         // Error message will include the custom message
///     }
///     _ => panic!("Expected validation error"),
/// }
/// ```
pub struct IPAddressValidator {
	allow_ipv4: bool,
	allow_ipv6: bool,
	message: Option<String>,
}

impl IPAddressValidator {
	/// Creates a new IPAddressValidator that accepts both IPv4 and IPv6 addresses.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::validators::{IPAddressValidator, Validator};
	///
	/// let validator = IPAddressValidator::new();
	/// assert!(validator.validate("192.168.1.1").is_ok());
	/// assert!(validator.validate("2001:db8::1").is_ok());
	/// ```
	pub fn new() -> Self {
		Self {
			allow_ipv4: true,
			allow_ipv6: true,
			message: None,
		}
	}

	/// Creates a validator that only accepts IPv4 addresses.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::validators::{IPAddressValidator, Validator};
	///
	/// let validator = IPAddressValidator::ipv4_only();
	/// assert!(validator.validate("192.168.1.1").is_ok());
	/// assert!(validator.validate("10.0.0.0").is_ok());
	/// assert!(validator.validate("2001:db8::1").is_err());
	/// ```
	pub fn ipv4_only() -> Self {
		Self {
			allow_ipv4: true,
			allow_ipv6: false,
			message: None,
		}
	}

	/// Creates a validator that only accepts IPv6 addresses.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::validators::{IPAddressValidator, Validator};
	///
	/// let validator = IPAddressValidator::ipv6_only();
	/// assert!(validator.validate("2001:db8::1").is_ok());
	/// assert!(validator.validate("::1").is_ok());
	/// assert!(validator.validate("192.168.1.1").is_err());
	/// ```
	pub fn ipv6_only() -> Self {
		Self {
			allow_ipv4: false,
			allow_ipv6: true,
			message: None,
		}
	}

	/// Sets a custom error message for validation failures.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::validators::{IPAddressValidator, Validator};
	///
	/// let validator = IPAddressValidator::new()
	///     .with_message("Invalid IP address format");
	///
	/// assert!(validator.validate("192.168.1.1").is_ok());
	/// ```
	pub fn with_message(mut self, message: impl Into<String>) -> Self {
		self.message = Some(message.into());
		self
	}

	/// Internal validation logic using std::net::IpAddr for type-safe parsing
	fn validate_ip(&self, value: &str) -> ValidationResult<()> {
		// Parse the IP address using std::net::IpAddr
		let ip_addr = value.parse::<IpAddr>().map_err(|_| {
			ValidationError::InvalidIPAddress(
				self.message
					.clone()
					.unwrap_or_else(|| "Invalid IP address format".to_string()),
			)
		})?;

		// Check if the parsed IP address type is allowed
		match ip_addr {
			IpAddr::V4(_) if !self.allow_ipv4 => Err(ValidationError::InvalidIPAddress(
				self.message
					.clone()
					.unwrap_or_else(|| "IPv4 addresses are not allowed".to_string()),
			)),
			IpAddr::V6(_) if !self.allow_ipv6 => Err(ValidationError::InvalidIPAddress(
				self.message
					.clone()
					.unwrap_or_else(|| "IPv6 addresses are not allowed".to_string()),
			)),
			_ => Ok(()),
		}
	}
}

impl Default for IPAddressValidator {
	fn default() -> Self {
		Self::new()
	}
}

impl Validator<String> for IPAddressValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		self.validate_ip(value.as_str())
	}
}

impl Validator<str> for IPAddressValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		self.validate_ip(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// Basic IPv4 validation tests
	#[rstest]
	fn test_ipv4_valid_addresses() {
		let validator = IPAddressValidator::new();
		let valid_ipv4 = vec![
			"0.0.0.0",
			"127.0.0.1",
			"192.168.1.1",
			"10.0.0.1",
			"172.16.0.1",
			"255.255.255.255",
			"8.8.8.8",
			"1.1.1.1",
		];

		for ip in valid_ipv4 {
			assert!(
				validator.validate(ip).is_ok(),
				"Expected {} to be valid IPv4",
				ip
			);
		}
	}

	#[rstest]
	fn test_ipv4_invalid_addresses() {
		let validator = IPAddressValidator::new();
		let invalid_ipv4 = vec![
			"256.1.1.1",      // Octet out of range
			"192.168.1.256",  // Octet out of range
			"192.168.1",      // Missing octet
			"192.168.1.1.1",  // Too many octets
			"192.168.-1.1",   // Negative number
			"192.168.1.a",    // Non-numeric
			"192.168..1",     // Empty octet
			"...",            // All empty
			"",               // Empty string
			"192.168.1.1/24", // CIDR notation not supported
		];

		for ip in invalid_ipv4 {
			assert!(
				validator.validate(ip).is_err(),
				"Expected {} to be invalid",
				ip
			);
		}
	}

	// Basic IPv6 validation tests
	#[rstest]
	fn test_ipv6_valid_addresses() {
		let validator = IPAddressValidator::new();
		let valid_ipv6 = vec![
			"::1",                                     // Loopback
			"::",                                      // All zeros
			"2001:db8::1",                             // Compressed
			"2001:0db8:85a3:0000:0000:8a2e:0370:7334", // Full form
			"2001:db8:85a3::8a2e:370:7334",            // Compressed middle
			"fe80::1",                                 // Link-local
			"::ffff:192.0.2.1",                        // IPv4-mapped IPv6
			"2001:db8::8a2e:370:7334",
			"2001:db8:0:0:1:0:0:1",
			"2001:0db8:0001:0000:0000:0ab9:C0A8:0102",
		];

		for ip in valid_ipv6 {
			assert!(
				validator.validate(ip).is_ok(),
				"Expected {} to be valid IPv6",
				ip
			);
		}
	}

	#[rstest]
	fn test_ipv6_invalid_addresses() {
		let validator = IPAddressValidator::new();
		let invalid_ipv6 = vec![
			"02001:db8::1",                        // Too many digits
			"2001:db8::1::2",                      // Double ::
			"gggg::1",                             // Invalid hex
			"2001:db8:85a3::8a2e:370k:7334",       // Invalid character
			"::1::2",                              // Multiple ::
			"2001:db8:85a3:8a2e:370:7334",         // Too few groups
			"2001:db8:85a3:0:0:8a2e:0:0:370:7334", // Too many groups
		];

		for ip in invalid_ipv6 {
			assert!(
				validator.validate(ip).is_err(),
				"Expected {} to be invalid",
				ip
			);
		}
	}

	// IPv4-only validator tests
	#[rstest]
	fn test_ipv4_only_validator() {
		let validator = IPAddressValidator::ipv4_only();

		// Should accept IPv4
		assert!(validator.validate("192.168.1.1").is_ok());
		assert!(validator.validate("10.0.0.1").is_ok());
		assert!(validator.validate("127.0.0.1").is_ok());

		// Should reject IPv6
		assert!(validator.validate("::1").is_err());
		assert!(validator.validate("2001:db8::1").is_err());
		assert!(validator.validate("fe80::1").is_err());
	}

	// IPv6-only validator tests
	#[rstest]
	fn test_ipv6_only_validator() {
		let validator = IPAddressValidator::ipv6_only();

		// Should accept IPv6
		assert!(validator.validate("::1").is_ok());
		assert!(validator.validate("2001:db8::1").is_ok());
		assert!(validator.validate("fe80::1").is_ok());

		// Should reject IPv4
		assert!(validator.validate("192.168.1.1").is_err());
		assert!(validator.validate("10.0.0.1").is_err());
		assert!(validator.validate("127.0.0.1").is_err());
	}

	// Custom message tests
	#[rstest]
	fn test_custom_error_message() {
		let custom_msg = "Please provide a valid IP address";
		let validator = IPAddressValidator::new().with_message(custom_msg);

		match validator.validate("invalid-ip") {
			Err(ValidationError::InvalidIPAddress(msg)) => {
				assert_eq!(msg, custom_msg);
			}
			_ => panic!("Expected InvalidIPAddress error with custom message"),
		}
	}

	#[rstest]
	fn test_custom_error_message_ipv4_only() {
		let custom_msg = "Only IPv4 addresses are allowed";
		let validator = IPAddressValidator::ipv4_only().with_message(custom_msg);

		match validator.validate("2001:db8::1") {
			Err(ValidationError::InvalidIPAddress(_)) => {
				// Custom message is used when IPv6 is not allowed
			}
			_ => panic!("Expected InvalidIPAddress error"),
		}
	}

	#[rstest]
	fn test_custom_error_message_ipv6_only() {
		let custom_msg = "Only IPv6 addresses are allowed";
		let validator = IPAddressValidator::ipv6_only().with_message(custom_msg);

		match validator.validate("192.168.1.1") {
			Err(ValidationError::InvalidIPAddress(_)) => {
				// Custom message is used when IPv4 is not allowed
			}
			_ => panic!("Expected InvalidIPAddress error"),
		}
	}

	// String type tests
	#[rstest]
	fn test_validator_with_string_type() {
		let validator = IPAddressValidator::new();

		let valid_ip = String::from("192.168.1.1");
		assert!(validator.validate(&valid_ip).is_ok());

		let invalid_ip = String::from("invalid");
		assert!(validator.validate(&invalid_ip).is_err());
	}

	#[rstest]
	fn test_validator_with_str_type() {
		let validator = IPAddressValidator::new();

		assert!(validator.validate("192.168.1.1").is_ok());
		assert!(validator.validate("2001:db8::1").is_ok());
		assert!(validator.validate("invalid").is_err());
	}

	// Edge cases
	#[rstest]
	fn test_empty_string() {
		let validator = IPAddressValidator::new();
		assert!(validator.validate("").is_err());
	}

	#[rstest]
	fn test_whitespace() {
		let validator = IPAddressValidator::new();
		assert!(validator.validate(" ").is_err());
		assert!(validator.validate("192.168.1.1 ").is_err());
		assert!(validator.validate(" 192.168.1.1").is_err());
	}

	#[rstest]
	fn test_special_ipv4_addresses() {
		let validator = IPAddressValidator::new();

		// Loopback
		assert!(validator.validate("127.0.0.1").is_ok());

		// Broadcast
		assert!(validator.validate("255.255.255.255").is_ok());

		// Network address
		assert!(validator.validate("0.0.0.0").is_ok());
	}

	#[rstest]
	fn test_special_ipv6_addresses() {
		let validator = IPAddressValidator::new();

		// Loopback
		assert!(validator.validate("::1").is_ok());

		// All zeros
		assert!(validator.validate("::").is_ok());

		// IPv4-mapped IPv6
		assert!(validator.validate("::ffff:192.0.2.1").is_ok());
	}

	#[rstest]
	fn test_ipv6_compression() {
		let validator = IPAddressValidator::new();

		// Various compression forms
		assert!(validator.validate("2001:db8::1").is_ok());
		assert!(validator.validate("2001:db8::").is_ok());
		assert!(validator.validate("::2001:db8:1").is_ok());
		assert!(validator.validate("2001:db8:0:0:0:0:0:1").is_ok());
	}

	#[rstest]
	fn test_default_implementation() {
		let validator = IPAddressValidator::default();
		assert!(validator.validate("192.168.1.1").is_ok());
		assert!(validator.validate("2001:db8::1").is_ok());
	}

	// Error type tests
	#[rstest]
	fn test_error_type_for_invalid_format() {
		let validator = IPAddressValidator::new();

		match validator.validate("not-an-ip") {
			Err(ValidationError::InvalidIPAddress(_)) => {}
			_ => panic!("Expected InvalidIPAddress error"),
		}
	}

	#[rstest]
	fn test_error_type_for_wrong_version() {
		let ipv4_validator = IPAddressValidator::ipv4_only();
		match ipv4_validator.validate("::1") {
			Err(ValidationError::InvalidIPAddress(_)) => {}
			_ => panic!("Expected InvalidIPAddress error"),
		}

		let ipv6_validator = IPAddressValidator::ipv6_only();
		match ipv6_validator.validate("192.168.1.1") {
			Err(ValidationError::InvalidIPAddress(_)) => {}
			_ => panic!("Expected InvalidIPAddress error"),
		}
	}

	// Real-world IP addresses
	#[rstest]
	fn test_real_world_ipv4_addresses() {
		let validator = IPAddressValidator::new();

		// Public DNS servers
		assert!(validator.validate("8.8.8.8").is_ok()); // Google DNS
		assert!(validator.validate("1.1.1.1").is_ok()); // Cloudflare DNS
		assert!(validator.validate("208.67.222.222").is_ok()); // OpenDNS

		// Private networks
		assert!(validator.validate("192.168.0.1").is_ok()); // Home router
		assert!(validator.validate("10.0.0.1").is_ok()); // Private network
		assert!(validator.validate("172.16.0.1").is_ok()); // Private network
	}

	#[rstest]
	fn test_real_world_ipv6_addresses() {
		let validator = IPAddressValidator::new();

		// Public IPv6 addresses
		assert!(validator.validate("2001:4860:4860::8888").is_ok()); // Google DNS
		assert!(validator.validate("2606:4700:4700::1111").is_ok()); // Cloudflare DNS

		// Link-local
		assert!(validator.validate("fe80::1").is_ok());
	}
}
