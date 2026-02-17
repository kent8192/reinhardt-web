//! Custom email headers for SMTP transport
//!
//! This module provides typed headers for common custom email headers.
//! Due to lettre's Header trait design (which requires a static `name()` method),
//! only pre-defined headers can be supported. Arbitrary custom headers cannot be
//! added dynamically at runtime.
//!
//! ## Supported Headers
//!
//! - `X-Mailer`: Identifies the email client/application
//! - `X-Priority`: Email priority (1=high, 3=normal, 5=low)
//! - `List-Unsubscribe`: URL for unsubscribe functionality
//! - `List-Unsubscribe-Post`: One-click unsubscribe support
//! - `X-Entity-Ref-ID`: Unique reference ID for tracking
//! - `Precedence`: Email precedence (bulk, list, junk)
//!
//! ## Limitations
//!
//! Lettre's `Header` trait requires the header name to be known at compile time
//! (the `name()` method is static, not `&self`). This means we cannot support
//! truly arbitrary custom headers - each header type must be pre-defined.
//! Unknown headers in `EmailMessage.headers` will be logged as warnings and skipped.

use lettre::message::header::{Header, HeaderName, HeaderValue};
use std::error::Error as StdError;

/// Macro to define a simple text-based header
macro_rules! define_text_header {
	($(#[$attr:meta])* $type_name:ident, $header_name:expr) => {
		$(#[$attr])*
		#[derive(Debug, Clone, PartialEq, Eq)]
		pub struct $type_name(String);

		impl $type_name {
			/// Create a new header with the given value
			pub fn new(value: impl Into<String>) -> Self {
				Self(value.into())
			}

			/// Get the header value
			pub fn value(&self) -> &str {
				&self.0
			}
		}

		impl Header for $type_name {
			fn name() -> HeaderName {
				HeaderName::new_from_ascii_str($header_name)
			}

			fn parse(s: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
				Ok(Self(s.to_string()))
			}

			fn display(&self) -> HeaderValue {
				HeaderValue::new(Self::name(), self.0.clone())
			}
		}

		impl From<String> for $type_name {
			fn from(s: String) -> Self {
				Self(s)
			}
		}

		impl From<&str> for $type_name {
			fn from(s: &str) -> Self {
				Self(s.to_string())
			}
		}
	};
}

define_text_header!(
	/// `X-Mailer` header - identifies the email client/application
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::XMailer;
	///
	/// let header = XMailer::new("Reinhardt Mail 1.0");
	/// assert_eq!(header.value(), "Reinhardt Mail 1.0");
	/// ```
	XMailer,
	"X-Mailer"
);

define_text_header!(
	/// `X-Priority` header - email priority level
	///
	/// Common values:
	/// - "1" = High
	/// - "3" = Normal
	/// - "5" = Low
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::XPriority;
	///
	/// let high = XPriority::new("1");
	/// let normal = XPriority::new("3");
	/// let low = XPriority::new("5");
	/// ```
	XPriority,
	"X-Priority"
);

define_text_header!(
	/// `List-Unsubscribe` header - provides unsubscribe URL
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::ListUnsubscribe;
	///
	/// let header = ListUnsubscribe::new("<https://example.com/unsubscribe?token=abc123>");
	/// ```
	ListUnsubscribe,
	"List-Unsubscribe"
);

define_text_header!(
	/// `List-Unsubscribe-Post` header - enables one-click unsubscribe
	///
	/// Typically set to "List-Unsubscribe=One-Click" for RFC 8058 compliance.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::ListUnsubscribePost;
	///
	/// let header = ListUnsubscribePost::new("List-Unsubscribe=One-Click");
	/// ```
	ListUnsubscribePost,
	"List-Unsubscribe-Post"
);

define_text_header!(
	/// `X-Entity-Ref-ID` header - unique reference ID for tracking
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::XEntityRefId;
	///
	/// let header = XEntityRefId::new("msg-12345-abcde");
	/// ```
	XEntityRefId,
	"X-Entity-Ref-ID"
);

define_text_header!(
	/// `Precedence` header - email precedence level
	///
	/// Common values:
	/// - "bulk" - bulk/marketing email
	/// - "list" - mailing list email
	/// - "junk" - low priority
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::headers::Precedence;
	///
	/// let header = Precedence::new("bulk");
	/// ```
	Precedence,
	"Precedence"
);

/// List of supported custom header names (lowercase for comparison)
pub const SUPPORTED_HEADERS: &[&str] = &[
	"x-mailer",
	"x-priority",
	"list-unsubscribe",
	"list-unsubscribe-post",
	"x-entity-ref-id",
	"precedence",
];

/// Check if a header name is supported
pub fn is_supported_header(name: &str) -> bool {
	SUPPORTED_HEADERS.contains(&name.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_x_mailer() {
		let header = XMailer::new("Test Mailer 1.0");
		assert_eq!(header.value(), "Test Mailer 1.0");
		assert_eq!(format!("{}", XMailer::name()), "X-Mailer");
	}

	#[rstest]
	fn test_x_priority() {
		let header = XPriority::new("1");
		assert_eq!(header.value(), "1");
		assert_eq!(format!("{}", XPriority::name()), "X-Priority");
	}

	#[rstest]
	fn test_list_unsubscribe() {
		let header = ListUnsubscribe::new("<https://example.com/unsubscribe>");
		assert_eq!(header.value(), "<https://example.com/unsubscribe>");
	}

	#[rstest]
	fn test_list_unsubscribe_post() {
		let header = ListUnsubscribePost::new("List-Unsubscribe=One-Click");
		assert_eq!(header.value(), "List-Unsubscribe=One-Click");
	}

	#[rstest]
	fn test_x_entity_ref_id() {
		let header = XEntityRefId::new("ref-123");
		assert_eq!(header.value(), "ref-123");
	}

	#[rstest]
	fn test_precedence() {
		let header = Precedence::new("bulk");
		assert_eq!(header.value(), "bulk");
	}

	#[rstest]
	fn test_is_supported_header() {
		assert!(is_supported_header("X-Mailer"));
		assert!(is_supported_header("x-mailer"));
		assert!(is_supported_header("X-MAILER"));
		assert!(is_supported_header("List-Unsubscribe"));
		assert!(!is_supported_header("X-Custom-Header"));
		assert!(!is_supported_header("Unknown-Header"));
	}

	#[rstest]
	fn test_from_string() {
		let header: XMailer = "Test".into();
		assert_eq!(header.value(), "Test");

		let header: XMailer = String::from("Test2").into();
		assert_eq!(header.value(), "Test2");
	}

	#[rstest]
	fn test_header_parse() {
		let header = XMailer::parse("Parsed Value").unwrap();
		assert_eq!(header.value(), "Parsed Value");
	}
}
