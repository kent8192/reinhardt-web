//! Asset bundling for desktop applications.

/// Type of asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
	/// CSS stylesheet.
	Css,
	/// JavaScript code.
	Js,
}

/// A single asset entry.
#[derive(Debug, Clone)]
pub struct AssetEntry {
	/// Type of the asset.
	pub asset_type: AssetType,
	/// Asset content.
	pub content: String,
	/// Optional source path for debugging.
	pub source_path: Option<String>,
}

impl AssetEntry {
	/// Creates a new asset entry.
	pub fn new(asset_type: AssetType, content: String, source_path: impl Into<String>) -> Self {
		Self {
			asset_type,
			content,
			source_path: Some(source_path.into()),
		}
	}

	/// Creates an inline asset without source path.
	pub fn inline(asset_type: AssetType, content: String) -> Self {
		Self {
			asset_type,
			content,
			source_path: None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_asset_entry_creation() {
		// Arrange
		let content = "body { color: red; }";

		// Act
		let entry = AssetEntry::new(AssetType::Css, content.to_string(), "main.css");

		// Assert
		assert_eq!(entry.asset_type, AssetType::Css);
		assert_eq!(entry.content, content);
		assert_eq!(entry.source_path, Some("main.css".to_string()));
	}

	#[rstest]
	fn test_asset_entry_inline() {
		// Arrange
		let content = "console.log('inline');";

		// Act
		let entry = AssetEntry::inline(AssetType::Js, content.to_string());

		// Assert
		assert_eq!(entry.asset_type, AssetType::Js);
		assert_eq!(entry.content, content);
		assert_eq!(entry.source_path, None);
	}
}
