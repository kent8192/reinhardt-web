//! Validator serialization support
//!
//! This module provides serializable representations of validator configurations,
//! allowing validators to be saved to and loaded from configuration files.
//!
//! # Feature Flag
//!
//! This module requires the `serde` feature to be enabled.
//!
//! # Examples
//!
//! ```
//! use reinhardt_core::validators::serialization::{ValidatorConfig, StringValidatorConfig};
//!
//! // Create a configuration
//! let config = ValidatorConfig::String(StringValidatorConfig {
//!     min_length: Some(3),
//!     max_length: Some(50),
//!     pattern: None,
//!     pattern_message: None,
//! });
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&config).unwrap();
//!
//! // Deserialize from JSON
//! let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();
//! ```

use serde::{Deserialize, Serialize};

/// Configuration for string validators
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StringValidatorConfig {
	/// Minimum length (inclusive)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_length: Option<usize>,

	/// Maximum length (inclusive)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_length: Option<usize>,

	/// Regex pattern for validation
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pattern: Option<String>,

	/// Custom message for pattern validation failure
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pattern_message: Option<String>,
}

/// Configuration for numeric validators
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NumericValidatorConfig {
	/// Minimum value (inclusive)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min: Option<f64>,

	/// Maximum value (inclusive)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max: Option<f64>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for email validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailValidatorConfig {
	/// Minimum length
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_length: Option<usize>,

	/// Maximum length
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_length: Option<usize>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for URL validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlValidatorConfig {
	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for IP address validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IpAddressValidatorConfig {
	/// Whether to allow IPv4 addresses
	#[serde(default = "default_true")]
	pub allow_ipv4: bool,

	/// Whether to allow IPv6 addresses
	#[serde(default = "default_true")]
	pub allow_ipv6: bool,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

fn default_true() -> bool {
	true
}

/// Configuration for phone number validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhoneNumberValidatorConfig {
	/// Allowed country codes (e.g., ["1", "81", "44"])
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_codes: Option<Vec<String>>,

	/// Whether to allow extension numbers
	#[serde(default)]
	pub allow_extensions: bool,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Color format for color validator
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorFormatConfig {
	/// Hex color format (#RGB, #RRGGBB, #RRGGBBAA)
	Hex,
	/// RGB color format (rgb(255, 255, 255))
	Rgb,
	/// RGBA color format (rgba(255, 255, 255, 1.0))
	Rgba,
	/// HSL color format (hsl(360, 100%, 50%))
	Hsl,
	/// HSLA color format (hsla(360, 100%, 50%, 1.0))
	Hsla,
	/// Any supported color format
	Any,
}

/// Configuration for color validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorValidatorConfig {
	/// Allowed color formats
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_formats: Option<Vec<ColorFormatConfig>>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for UUID validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UuidValidatorConfig {
	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for slug validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlugValidatorConfig {
	/// Whether to allow unicode characters
	#[serde(default)]
	pub allow_unicode: bool,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for date/time validators
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DateTimeValidatorConfig {
	/// Format string (e.g., "%Y-%m-%d", "%H:%M:%S")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub format: Option<String>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for file type validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileTypeValidatorConfig {
	/// Allowed file extensions (e.g., ["jpg", "png", "gif"])
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_extensions: Option<Vec<String>>,

	/// Allowed MIME types (e.g., ["image/jpeg", "image/png"])
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_mime_types: Option<Vec<String>>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for file size validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileSizeValidatorConfig {
	/// Maximum file size in bytes
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_size: Option<u64>,

	/// Minimum file size in bytes
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_size: Option<u64>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Configuration for image dimension validator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageDimensionValidatorConfig {
	/// Minimum width in pixels
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_width: Option<u32>,

	/// Maximum width in pixels
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_width: Option<u32>,

	/// Minimum height in pixels
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_height: Option<u32>,

	/// Maximum height in pixels
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_height: Option<u32>,

	/// Custom error message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

/// Unified validator configuration enum
///
/// This enum represents all supported validator types and their configurations.
/// It can be serialized to and deserialized from JSON or other formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidatorConfig {
	/// String length and pattern validation
	String(StringValidatorConfig),

	/// Numeric range validation
	Numeric(NumericValidatorConfig),

	/// Email format validation
	Email(EmailValidatorConfig),

	/// URL format validation
	Url(UrlValidatorConfig),

	/// IP address validation
	IpAddress(IpAddressValidatorConfig),

	/// Phone number validation
	PhoneNumber(PhoneNumberValidatorConfig),

	/// Color format validation
	Color(ColorValidatorConfig),

	/// UUID format validation
	Uuid(UuidValidatorConfig),

	/// Slug format validation
	Slug(SlugValidatorConfig),

	/// Date format validation
	Date(DateTimeValidatorConfig),

	/// Time format validation
	Time(DateTimeValidatorConfig),

	/// DateTime format validation
	DateTime(DateTimeValidatorConfig),

	/// File type validation
	FileType(FileTypeValidatorConfig),

	/// File size validation
	FileSize(FileSizeValidatorConfig),

	/// Image dimension validation
	ImageDimension(ImageDimensionValidatorConfig),
}

impl ValidatorConfig {
	/// Create a string validator configuration
	pub fn string() -> StringValidatorConfig {
		StringValidatorConfig::default()
	}

	/// Create a numeric validator configuration
	pub fn numeric() -> NumericValidatorConfig {
		NumericValidatorConfig::default()
	}

	/// Create an email validator configuration
	pub fn email() -> EmailValidatorConfig {
		EmailValidatorConfig::default()
	}

	/// Create a URL validator configuration
	pub fn url() -> UrlValidatorConfig {
		UrlValidatorConfig::default()
	}

	/// Create an IP address validator configuration
	pub fn ip_address() -> IpAddressValidatorConfig {
		IpAddressValidatorConfig {
			allow_ipv4: true,
			allow_ipv6: true,
			message: None,
		}
	}

	/// Create a phone number validator configuration
	pub fn phone_number() -> PhoneNumberValidatorConfig {
		PhoneNumberValidatorConfig::default()
	}

	/// Create a color validator configuration
	pub fn color() -> ColorValidatorConfig {
		ColorValidatorConfig::default()
	}

	/// Create a UUID validator configuration
	pub fn uuid() -> UuidValidatorConfig {
		UuidValidatorConfig::default()
	}

	/// Create a slug validator configuration
	pub fn slug() -> SlugValidatorConfig {
		SlugValidatorConfig::default()
	}
}

/// Builder pattern for StringValidatorConfig
impl StringValidatorConfig {
	/// Set minimum length
	pub fn min_length(mut self, len: usize) -> Self {
		self.min_length = Some(len);
		self
	}

	/// Set maximum length
	pub fn max_length(mut self, len: usize) -> Self {
		self.max_length = Some(len);
		self
	}

	/// Set regex pattern
	pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
		self.pattern = Some(pattern.into());
		self
	}

	/// Set custom pattern error message
	pub fn pattern_message(mut self, message: impl Into<String>) -> Self {
		self.pattern_message = Some(message.into());
		self
	}

	/// Build into a ValidatorConfig
	pub fn build(self) -> ValidatorConfig {
		ValidatorConfig::String(self)
	}
}

/// Builder pattern for NumericValidatorConfig
impl NumericValidatorConfig {
	/// Set minimum value
	pub fn min(mut self, value: f64) -> Self {
		self.min = Some(value);
		self
	}

	/// Set maximum value
	pub fn max(mut self, value: f64) -> Self {
		self.max = Some(value);
		self
	}

	/// Set custom error message
	pub fn message(mut self, msg: impl Into<String>) -> Self {
		self.message = Some(msg.into());
		self
	}

	/// Build into a ValidatorConfig
	pub fn build(self) -> ValidatorConfig {
		ValidatorConfig::Numeric(self)
	}
}

/// Builder pattern for EmailValidatorConfig
impl EmailValidatorConfig {
	/// Set minimum length
	pub fn min_length(mut self, len: usize) -> Self {
		self.min_length = Some(len);
		self
	}

	/// Set maximum length
	pub fn max_length(mut self, len: usize) -> Self {
		self.max_length = Some(len);
		self
	}

	/// Set custom error message
	pub fn message(mut self, msg: impl Into<String>) -> Self {
		self.message = Some(msg.into());
		self
	}

	/// Build into a ValidatorConfig
	pub fn build(self) -> ValidatorConfig {
		ValidatorConfig::Email(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_string_config_serialization() {
		let config = ValidatorConfig::String(StringValidatorConfig {
			min_length: Some(3),
			max_length: Some(50),
			pattern: Some(r"^[a-z]+$".into()),
			pattern_message: Some("Only lowercase letters allowed".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::String(cfg) => {
				assert_eq!(cfg.min_length, Some(3));
				assert_eq!(cfg.max_length, Some(50));
				assert_eq!(cfg.pattern, Some(r"^[a-z]+$".into()));
			}
			_ => panic!("Expected String config"),
		}
	}

	#[test]
	fn test_numeric_config_serialization() {
		let config = ValidatorConfig::Numeric(NumericValidatorConfig {
			min: Some(0.0),
			max: Some(100.0),
			message: Some("Value must be between 0 and 100".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::Numeric(cfg) => {
				assert_eq!(cfg.min, Some(0.0));
				assert_eq!(cfg.max, Some(100.0));
			}
			_ => panic!("Expected Numeric config"),
		}
	}

	#[test]
	fn test_email_config_serialization() {
		let config = ValidatorConfig::Email(EmailValidatorConfig {
			min_length: Some(5),
			max_length: Some(254),
			message: None,
		});

		let json = serde_json::to_string(&config).unwrap();
		assert!(json.contains("\"type\":\"email\""));

		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();
		match loaded {
			ValidatorConfig::Email(cfg) => {
				assert_eq!(cfg.min_length, Some(5));
				assert_eq!(cfg.max_length, Some(254));
			}
			_ => panic!("Expected Email config"),
		}
	}

	#[test]
	fn test_ip_address_config_defaults() {
		let json = r#"{"type":"ip_address"}"#;
		let config: ValidatorConfig = serde_json::from_str(json).unwrap();

		match config {
			ValidatorConfig::IpAddress(cfg) => {
				assert!(cfg.allow_ipv4);
				assert!(cfg.allow_ipv6);
			}
			_ => panic!("Expected IpAddress config"),
		}
	}

	#[test]
	fn test_phone_number_config_serialization() {
		let config = ValidatorConfig::PhoneNumber(PhoneNumberValidatorConfig {
			country_codes: Some(vec!["1".into(), "81".into(), "44".into()]),
			allow_extensions: true,
			message: None,
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::PhoneNumber(cfg) => {
				assert_eq!(cfg.country_codes.as_ref().unwrap().len(), 3);
				assert!(cfg.allow_extensions);
			}
			_ => panic!("Expected PhoneNumber config"),
		}
	}

	#[test]
	fn test_color_config_serialization() {
		let config = ValidatorConfig::Color(ColorValidatorConfig {
			allowed_formats: Some(vec![ColorFormatConfig::Hex, ColorFormatConfig::Rgb]),
			message: Some("Use hex or RGB format".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		assert!(json.contains("\"hex\""));
		assert!(json.contains("\"rgb\""));

		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();
		match loaded {
			ValidatorConfig::Color(cfg) => {
				let formats = cfg.allowed_formats.unwrap();
				assert_eq!(formats.len(), 2);
				assert!(formats.contains(&ColorFormatConfig::Hex));
				assert!(formats.contains(&ColorFormatConfig::Rgb));
			}
			_ => panic!("Expected Color config"),
		}
	}

	#[test]
	fn test_file_type_config_serialization() {
		let config = ValidatorConfig::FileType(FileTypeValidatorConfig {
			allowed_extensions: Some(vec!["jpg".into(), "png".into()]),
			allowed_mime_types: Some(vec!["image/jpeg".into(), "image/png".into()]),
			message: None,
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::FileType(cfg) => {
				assert_eq!(cfg.allowed_extensions.as_ref().unwrap().len(), 2);
				assert_eq!(cfg.allowed_mime_types.as_ref().unwrap().len(), 2);
			}
			_ => panic!("Expected FileType config"),
		}
	}

	#[test]
	fn test_image_dimension_config_serialization() {
		let config = ValidatorConfig::ImageDimension(ImageDimensionValidatorConfig {
			min_width: Some(100),
			max_width: Some(1920),
			min_height: Some(100),
			max_height: Some(1080),
			message: None,
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::ImageDimension(cfg) => {
				assert_eq!(cfg.min_width, Some(100));
				assert_eq!(cfg.max_width, Some(1920));
				assert_eq!(cfg.min_height, Some(100));
				assert_eq!(cfg.max_height, Some(1080));
			}
			_ => panic!("Expected ImageDimension config"),
		}
	}

	#[test]
	fn test_builder_pattern() {
		let config = ValidatorConfig::string()
			.min_length(1)
			.max_length(100)
			.pattern(r"^[a-zA-Z]+$")
			.pattern_message("Only letters allowed")
			.build();

		match config {
			ValidatorConfig::String(cfg) => {
				assert_eq!(cfg.min_length, Some(1));
				assert_eq!(cfg.max_length, Some(100));
				assert!(cfg.pattern.is_some());
				assert!(cfg.pattern_message.is_some());
			}
			_ => panic!("Expected String config"),
		}
	}

	#[test]
	fn test_numeric_builder() {
		let config = ValidatorConfig::numeric()
			.min(0.0)
			.max(100.0)
			.message("Value out of range")
			.build();

		match config {
			ValidatorConfig::Numeric(cfg) => {
				assert_eq!(cfg.min, Some(0.0));
				assert_eq!(cfg.max, Some(100.0));
				assert!(cfg.message.is_some());
			}
			_ => panic!("Expected Numeric config"),
		}
	}

	#[test]
	fn test_email_builder() {
		let config = ValidatorConfig::email()
			.min_length(5)
			.max_length(254)
			.message("Invalid email")
			.build();

		match config {
			ValidatorConfig::Email(cfg) => {
				assert_eq!(cfg.min_length, Some(5));
				assert_eq!(cfg.max_length, Some(254));
				assert!(cfg.message.is_some());
			}
			_ => panic!("Expected Email config"),
		}
	}

	#[test]
	fn test_skip_serializing_none_fields() {
		let config = ValidatorConfig::String(StringValidatorConfig {
			min_length: Some(1),
			max_length: None,
			pattern: None,
			pattern_message: None,
		});

		let json = serde_json::to_string(&config).unwrap();

		// None fields should not be in the JSON
		assert!(json.contains("\"min_length\":1"));
		assert!(!json.contains("\"max_length\""));
		assert!(!json.contains("\"pattern\""));
		assert!(!json.contains("\"pattern_message\""));
	}

	#[test]
	fn test_deserialize_minimal_config() {
		// Minimal config with only type and required fields
		let json = r#"{"type":"string"}"#;
		let config: ValidatorConfig = serde_json::from_str(json).unwrap();

		match config {
			ValidatorConfig::String(cfg) => {
				assert!(cfg.min_length.is_none());
				assert!(cfg.max_length.is_none());
				assert!(cfg.pattern.is_none());
			}
			_ => panic!("Expected String config"),
		}
	}

	#[test]
	fn test_slug_config() {
		let config = ValidatorConfig::Slug(SlugValidatorConfig {
			allow_unicode: true,
			message: Some("Invalid slug".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::Slug(cfg) => {
				assert!(cfg.allow_unicode);
				assert!(cfg.message.is_some());
			}
			_ => panic!("Expected Slug config"),
		}
	}

	#[test]
	fn test_uuid_config() {
		let config = ValidatorConfig::Uuid(UuidValidatorConfig {
			message: Some("Invalid UUID format".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::Uuid(cfg) => {
				assert_eq!(cfg.message, Some("Invalid UUID format".into()));
			}
			_ => panic!("Expected UUID config"),
		}
	}

	#[test]
	fn test_file_size_config() {
		let config = ValidatorConfig::FileSize(FileSizeValidatorConfig {
			max_size: Some(10 * 1024 * 1024), // 10 MB
			min_size: Some(1024),             // 1 KB
			message: None,
		});

		let json = serde_json::to_string(&config).unwrap();
		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();

		match loaded {
			ValidatorConfig::FileSize(cfg) => {
				assert_eq!(cfg.max_size, Some(10 * 1024 * 1024));
				assert_eq!(cfg.min_size, Some(1024));
			}
			_ => panic!("Expected FileSize config"),
		}
	}

	#[test]
	fn test_url_config() {
		let config = ValidatorConfig::Url(UrlValidatorConfig {
			message: Some("Invalid URL".into()),
		});

		let json = serde_json::to_string(&config).unwrap();
		assert!(json.contains("\"type\":\"url\""));

		let loaded: ValidatorConfig = serde_json::from_str(&json).unwrap();
		match loaded {
			ValidatorConfig::Url(cfg) => {
				assert!(cfg.message.is_some());
			}
			_ => panic!("Expected URL config"),
		}
	}
}
