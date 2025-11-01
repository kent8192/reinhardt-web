//! Source map generation for minified assets
//!
//! Generates source maps to help with debugging minified JavaScript and CSS files.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Source map for minified files
///
/// Follows the Source Map Revision 3 specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
	/// Source map version (always 3)
	pub version: u8,
	/// Output file name
	pub file: String,
	/// Source files
	pub sources: Vec<String>,
	/// Source file contents (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sources_content: Option<Vec<String>>,
	/// Variable/property names
	pub names: Vec<String>,
	/// Mappings string (VLQ encoded)
	pub mappings: String,
	/// Source root (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source_root: Option<String>,
}

impl SourceMap {
	/// Create a new source map
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMap;
	///
	/// let source_map = SourceMap::new("app.min.js".to_string());
	/// assert_eq!(source_map.version, 3);
	/// assert_eq!(source_map.file, "app.min.js");
	/// ```
	pub fn new(file: String) -> Self {
		Self {
			version: 3,
			file,
			sources: Vec::new(),
			sources_content: None,
			names: Vec::new(),
			mappings: String::new(),
			source_root: None,
		}
	}

	/// Add a source file
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMap;
	///
	/// let mut source_map = SourceMap::new("app.min.js".to_string());
	/// source_map.add_source("src/app.js".to_string());
	/// assert_eq!(source_map.sources.len(), 1);
	/// ```
	pub fn add_source(&mut self, source: String) {
		self.sources.push(source);
	}

	/// Add source content
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMap;
	///
	/// let mut source_map = SourceMap::new("app.min.js".to_string());
	/// source_map.add_source("src/app.js".to_string());
	/// source_map.add_source_content("const x = 1;".to_string());
	/// ```
	pub fn add_source_content(&mut self, content: String) {
		if self.sources_content.is_none() {
			self.sources_content = Some(Vec::new());
		}
		self.sources_content.as_mut().unwrap().push(content);
	}

	/// Add a name
	pub fn add_name(&mut self, name: String) {
		self.names.push(name);
	}

	/// Set mappings
	pub fn set_mappings(&mut self, mappings: String) {
		self.mappings = mappings;
	}

	/// Set source root
	pub fn set_source_root(&mut self, root: String) {
		self.source_root = Some(root);
	}

	/// Convert to JSON string
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMap;
	///
	/// let source_map = SourceMap::new("app.min.js".to_string());
	/// let json = source_map.to_json().unwrap();
	/// assert!(json.contains("\"version\":3"));
	/// ```
	pub fn to_json(&self) -> io::Result<String> {
		serde_json::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}

	/// Convert to JSON string with pretty formatting
	pub fn to_json_pretty(&self) -> io::Result<String> {
		serde_json::to_string_pretty(self)
			.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}

	/// Load from JSON string
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMap;
	///
	/// let json = r#"{"version":3,"file":"app.min.js","sources":[],"names":[],"mappings":""}"#;
	/// let source_map = SourceMap::from_json(json).unwrap();
	/// assert_eq!(source_map.version, 3);
	/// ```
	pub fn from_json(json: &str) -> io::Result<Self> {
		serde_json::from_str(json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}
}

/// Source map generator
///
/// Generates source maps for minified files.
pub struct SourceMapGenerator {
	/// Enable inline source content
	inline_sources: bool,
	/// Source root path
	source_root: Option<String>,
}

impl SourceMapGenerator {
	/// Create a new source map generator
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMapGenerator;
	///
	/// let generator = SourceMapGenerator::new();
	/// ```
	pub fn new() -> Self {
		Self {
			inline_sources: true,
			source_root: None,
		}
	}

	/// Enable or disable inline source content
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMapGenerator;
	///
	/// let generator = SourceMapGenerator::new().with_inline_sources(false);
	/// ```
	pub fn with_inline_sources(mut self, enable: bool) -> Self {
		self.inline_sources = enable;
		self
	}

	/// Set source root
	pub fn with_source_root(mut self, root: String) -> Self {
		self.source_root = Some(root);
		self
	}

	/// Generate a basic source map from file path
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMapGenerator;
	/// use std::path::Path;
	///
	/// let generator = SourceMapGenerator::new();
	/// let source_map = generator.generate_for_file(
	///     Path::new("app.min.js"),
	///     Path::new("src/app.js"),
	///     "const x = 1;"
	/// );
	/// assert_eq!(source_map.file, "app.min.js");
	/// ```
	pub fn generate_for_file(
		&self,
		output_path: &Path,
		source_path: &Path,
		source_content: &str,
	) -> SourceMap {
		let mut map = SourceMap::new(
			output_path
				.file_name()
				.and_then(|n| n.to_str())
				.unwrap_or("output.js")
				.to_string(),
		);

		map.add_source(source_path.to_str().unwrap_or("source.js").to_string());

		if self.inline_sources {
			map.add_source_content(source_content.to_string());
		}

		if let Some(ref root) = self.source_root {
			map.set_source_root(root.clone());
		}

		// Basic identity mapping (every line maps to itself)
		// In real implementation, this would be generated during minification
		map.set_mappings("AAAA".to_string());

		map
	}

	/// Generate source map comment for inclusion in minified file
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::sourcemap::SourceMapGenerator;
	///
	/// let generator = SourceMapGenerator::new();
	/// let comment = generator.generate_comment("app.min.js.map");
	/// assert!(comment.contains("//# sourceMappingURL="));
	/// ```
	pub fn generate_comment(&self, map_filename: &str) -> String {
		format!("//# sourceMappingURL={}", map_filename)
	}

	/// Generate inline source map comment
	pub fn generate_inline_comment(&self, map: &SourceMap) -> io::Result<String> {
		let json = map.to_json()?;
		let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json);
		Ok(format!(
			"//# sourceMappingURL=data:application/json;base64,{}",
			encoded
		))
	}
}

impl Default for SourceMapGenerator {
	fn default() -> Self {
		Self::new()
	}
}

/// Source map merger
///
/// Merges multiple source maps together.
pub struct SourceMapMerger {
	maps: Vec<SourceMap>,
}

impl SourceMapMerger {
	/// Create a new source map merger
	pub fn new() -> Self {
		Self { maps: Vec::new() }
	}

	/// Add a source map
	pub fn add_map(&mut self, map: SourceMap) {
		self.maps.push(map);
	}

	/// Merge all source maps into one
	pub fn merge(&self, output_file: String) -> SourceMap {
		let mut merged = SourceMap::new(output_file);

		for map in &self.maps {
			for source in &map.sources {
				merged.add_source(source.clone());
			}
			if let Some(ref contents) = map.sources_content {
				for content in contents {
					merged.add_source_content(content.clone());
				}
			}
			for name in &map.names {
				merged.add_name(name.clone());
			}
		}

		// In real implementation, mappings would be properly merged
		merged.set_mappings("AAAA".to_string());

		merged
	}
}

impl Default for SourceMapMerger {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn test_source_map_creation() {
		let map = SourceMap::new("app.min.js".to_string());
		assert_eq!(map.version, 3);
		assert_eq!(map.file, "app.min.js");
		assert!(map.sources.is_empty());
		assert!(map.names.is_empty());
		assert_eq!(map.mappings, "");
	}

	#[test]
	fn test_add_source() {
		let mut map = SourceMap::new("app.min.js".to_string());
		map.add_source("src/app.js".to_string());
		assert_eq!(map.sources.len(), 1);
		assert_eq!(map.sources[0], "src/app.js");
	}

	#[test]
	fn test_add_source_content() {
		let mut map = SourceMap::new("app.min.js".to_string());
		map.add_source("src/app.js".to_string());
		map.add_source_content("const x = 1;".to_string());
		assert!(map.sources_content.is_some());
		assert_eq!(map.sources_content.as_ref().unwrap().len(), 1);
	}

	#[test]
	fn test_to_json() {
		let map = SourceMap::new("app.min.js".to_string());
		let json = map.to_json().unwrap();
		assert!(json.contains("\"version\":3"));
		assert!(json.contains("\"file\":\"app.min.js\""));
	}

	#[test]
	fn test_from_json() {
		let json = r#"{"version":3,"file":"app.min.js","sources":["src/app.js"],"names":[],"mappings":"AAAA"}"#;
		let map = SourceMap::from_json(json).unwrap();
		assert_eq!(map.version, 3);
		assert_eq!(map.file, "app.min.js");
		assert_eq!(map.sources.len(), 1);
	}

	#[test]
	fn test_generator_creation() {
		let generator = SourceMapGenerator::new();
		assert!(generator.inline_sources);
		assert!(generator.source_root.is_none());
	}

	#[test]
	fn test_generator_with_settings() {
		let generator = SourceMapGenerator::new()
			.with_inline_sources(false)
			.with_source_root("/src".to_string());
		assert!(!generator.inline_sources);
		assert_eq!(generator.source_root.unwrap(), "/src");
	}

	#[test]
	fn test_generate_for_file() {
		let generator = SourceMapGenerator::new();
		let map = generator.generate_for_file(
			&PathBuf::from("dist/app.min.js"),
			&PathBuf::from("src/app.js"),
			"const x = 1;",
		);
		assert_eq!(map.file, "app.min.js");
		assert_eq!(map.sources.len(), 1);
		assert!(map.sources_content.is_some());
	}

	#[test]
	fn test_generate_comment() {
		let generator = SourceMapGenerator::new();
		let comment = generator.generate_comment("app.min.js.map");
		assert_eq!(comment, "//# sourceMappingURL=app.min.js.map");
	}

	#[test]
	fn test_merger_creation() {
		let merger = SourceMapMerger::new();
		assert_eq!(merger.maps.len(), 0);
	}

	#[test]
	fn test_merger_add_map() {
		let mut merger = SourceMapMerger::new();
		let map = SourceMap::new("app.min.js".to_string());
		merger.add_map(map);
		assert_eq!(merger.maps.len(), 1);
	}

	#[test]
	fn test_merger_merge() {
		let mut merger = SourceMapMerger::new();

		let mut map1 = SourceMap::new("app1.min.js".to_string());
		map1.add_source("src/app1.js".to_string());
		merger.add_map(map1);

		let mut map2 = SourceMap::new("app2.min.js".to_string());
		map2.add_source("src/app2.js".to_string());
		merger.add_map(map2);

		let merged = merger.merge("bundle.min.js".to_string());
		assert_eq!(merged.file, "bundle.min.js");
		assert_eq!(merged.sources.len(), 2);
	}
}
