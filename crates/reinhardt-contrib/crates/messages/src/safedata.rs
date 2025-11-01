//! SafeData type for marking strings as HTML-safe

use serde::{Deserialize, Serialize};
use std::fmt;

/// A string wrapper that marks content as HTML-safe
///
/// This type is used to indicate that a string contains HTML that has been
/// sanitized or is otherwise safe to render without escaping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeData {
	content: String,
}

impl SafeData {
	/// Create a new SafeData instance
	///
	/// # Safety
	/// The caller must ensure that the content is actually safe HTML
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_messages::SafeData;
	///
	/// let safe = SafeData::new("<b>Bold text</b>");
	/// assert_eq!(safe.as_str(), "<b>Bold text</b>");
	/// ```
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
		}
	}
	/// Get the underlying content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_messages::SafeData;
	///
	/// let safe = SafeData::new("<p>Paragraph</p>");
	/// assert_eq!(safe.as_str(), "<p>Paragraph</p>");
	/// ```
	pub fn as_str(&self) -> &str {
		&self.content
	}
	/// Convert into the underlying String
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_messages::SafeData;
	///
	/// let safe = SafeData::new("<div>Content</div>");
	/// let string = safe.into_string();
	/// assert_eq!(string, "<div>Content</div>");
	/// ```
	pub fn into_string(self) -> String {
		self.content
	}
}

impl fmt::Display for SafeData {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.content)
	}
}

impl From<String> for SafeData {
	fn from(s: String) -> Self {
		Self::new(s)
	}
}

impl From<&str> for SafeData {
	fn from(s: &str) -> Self {
		Self::new(s)
	}
}

impl AsRef<str> for SafeData {
	fn as_ref(&self) -> &str {
		&self.content
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_safedata_creation() {
		let safe = SafeData::new("<b>Bold</b>");
		assert_eq!(safe.as_str(), "<b>Bold</b>");
	}

	#[test]
	fn test_safedata_display() {
		let safe = SafeData::new("<i>Italic</i>");
		assert_eq!(format!("{}", safe), "<i>Italic</i>");
	}

	#[test]
	fn test_safedata_serialization() {
		let safe = SafeData::new("<p>Test</p>");
		let json = serde_json::to_string(&safe).unwrap();
		let deserialized: SafeData = serde_json::from_str(&json).unwrap();
		assert_eq!(safe, deserialized);
	}

	#[test]
	fn test_safedata_from_string() {
		let s = String::from("<div>Content</div>");
		let safe: SafeData = s.into();
		assert_eq!(safe.as_str(), "<div>Content</div>");
	}

	#[test]
	fn test_safedata_from_str() {
		let safe: SafeData = "<span>Text</span>".into();
		assert_eq!(safe.as_str(), "<span>Text</span>");
	}
}
