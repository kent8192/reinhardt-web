//! Asset bundling for desktop applications.

use crate::error::Result;

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

/// Collects assets from IR traversal.
#[derive(Debug, Default)]
pub struct AssetCollector {
	assets: Vec<AssetEntry>,
}

impl AssetCollector {
	/// Creates a new asset collector.
	pub fn new() -> Self {
		Self::default()
	}

	/// Adds a CSS asset.
	pub fn add_css(&mut self, content: impl Into<String>, source: Option<&str>) {
		let entry = match source {
			Some(s) => AssetEntry::new(AssetType::Css, content.into(), s),
			None => AssetEntry::inline(AssetType::Css, content.into()),
		};
		self.assets.push(entry);
	}

	/// Adds a JavaScript asset.
	pub fn add_js(&mut self, content: impl Into<String>, source: Option<&str>) {
		let entry = match source {
			Some(s) => AssetEntry::new(AssetType::Js, content.into(), s),
			None => AssetEntry::inline(AssetType::Js, content.into()),
		};
		self.assets.push(entry);
	}

	/// Gets all assets of a specific type.
	pub fn get_assets(&self, asset_type: AssetType) -> Vec<&AssetEntry> {
		self.assets
			.iter()
			.filter(|a| a.asset_type == asset_type)
			.collect()
	}

	/// Consumes the collector and returns all assets.
	pub fn into_assets(self) -> Vec<AssetEntry> {
		self.assets
	}
}

/// Configuration for asset bundling.
#[derive(Debug, Clone, Default)]
pub struct BundleConfig {
	/// Whether to minify CSS.
	pub minify_css: bool,
	/// Whether to minify JS.
	pub minify_js: bool,
	/// Whether to add source maps.
	pub source_maps: bool,
}

/// Bundles assets for desktop applications.
#[derive(Debug)]
pub struct DesktopBundler {
	#[allow(dead_code)] // Will be used for minification in the future
	config: BundleConfig,
}

impl DesktopBundler {
	/// Creates a new bundler with default configuration.
	pub fn new() -> Self {
		Self {
			config: BundleConfig::default(),
		}
	}

	/// Creates a bundler with custom configuration.
	pub fn with_config(config: BundleConfig) -> Self {
		Self { config }
	}

	/// Bundles all CSS assets into a single string.
	pub fn bundle_css(&self, collector: &AssetCollector) -> Result<String> {
		let assets = collector.get_assets(AssetType::Css);
		let combined: String = assets
			.iter()
			.map(|a| a.content.as_str())
			.collect::<Vec<_>>()
			.join("\n");
		Ok(combined)
	}

	/// Bundles all JS assets into a single string.
	pub fn bundle_js(&self, collector: &AssetCollector) -> Result<String> {
		let assets = collector.get_assets(AssetType::Js);
		let combined: String = assets
			.iter()
			.map(|a| a.content.as_str())
			.collect::<Vec<_>>()
			.join(";\n");
		Ok(combined)
	}
}

impl Default for DesktopBundler {
	fn default() -> Self {
		Self::new()
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

	#[rstest]
	fn test_asset_collector_add_css() {
		// Arrange
		let mut collector = AssetCollector::new();

		// Act
		collector.add_css("body { margin: 0; }", Some("reset.css"));

		// Assert
		let assets = collector.get_assets(AssetType::Css);
		assert_eq!(assets.len(), 1);
		assert_eq!(assets[0].content, "body { margin: 0; }");
	}

	#[rstest]
	fn test_asset_collector_add_js() {
		// Arrange
		let mut collector = AssetCollector::new();

		// Act
		collector.add_js("console.log('hello');", Some("main.js"));

		// Assert
		let assets = collector.get_assets(AssetType::Js);
		assert_eq!(assets.len(), 1);
		assert_eq!(assets[0].content, "console.log('hello');");
	}

	#[rstest]
	fn test_asset_collector_multiple_assets() {
		// Arrange
		let mut collector = AssetCollector::new();

		// Act
		collector.add_css(".a { }", None);
		collector.add_css(".b { }", None);
		collector.add_js("const x = 1;", None);

		// Assert
		assert_eq!(collector.get_assets(AssetType::Css).len(), 2);
		assert_eq!(collector.get_assets(AssetType::Js).len(), 1);
	}

	#[rstest]
	fn test_desktop_bundler_bundle_css() {
		// Arrange
		let mut collector = AssetCollector::new();
		collector.add_css(".a { color: red; }", None);
		collector.add_css(".b { color: blue; }", None);

		let bundler = DesktopBundler::new();

		// Act
		let result = bundler.bundle_css(&collector);

		// Assert
		assert!(result.is_ok());
		let bundled = result.unwrap();
		assert!(bundled.contains(".a { color: red; }"));
		assert!(bundled.contains(".b { color: blue; }"));
	}

	#[rstest]
	fn test_desktop_bundler_bundle_js() {
		// Arrange
		let mut collector = AssetCollector::new();
		collector.add_js("const a = 1", None);
		collector.add_js("const b = 2", None);

		let bundler = DesktopBundler::new();

		// Act
		let result = bundler.bundle_js(&collector);

		// Assert
		assert!(result.is_ok());
		let bundled = result.unwrap();
		assert!(bundled.contains("const a = 1"));
		assert!(bundled.contains("const b = 2"));
	}

	#[rstest]
	fn test_desktop_bundler_with_config() {
		// Arrange
		let config = BundleConfig {
			minify_css: true,
			minify_js: true,
			source_maps: false,
		};

		// Act
		let bundler = DesktopBundler::with_config(config);

		// Assert
		assert!(format!("{:?}", bundler).contains("DesktopBundler"));
	}
}
