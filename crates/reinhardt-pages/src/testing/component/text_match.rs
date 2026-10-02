//! Text matching support for native component queries.

/// Exact text matcher used by native component queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMatch {
	needle: String,
}

impl TextMatch {
	/// Creates a new exact text matcher.
	pub fn exact(text: impl Into<String>) -> Self {
		Self {
			needle: text.into(),
		}
	}

	pub(crate) fn matches(&self, text: &str) -> bool {
		text == self.needle
	}
}

impl From<&str> for TextMatch {
	fn from(value: &str) -> Self {
		Self::exact(value)
	}
}

impl From<String> for TextMatch {
	fn from(value: String) -> Self {
		Self::exact(value)
	}
}

impl From<&String> for TextMatch {
	fn from(value: &String) -> Self {
		Self::exact(value.clone())
	}
}
