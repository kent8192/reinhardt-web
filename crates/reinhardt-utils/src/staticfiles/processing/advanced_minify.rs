//! Advanced JavaScript minification with OXC
//!
//! Provides production-grade minification with:
//! - Variable renaming (mangling)
//! - Dead code elimination
//! - Tree shaking
//! - Advanced compression optimizations

use super::{ProcessingResult, Processor};
use async_trait::async_trait;
use std::io;
use std::path::Path;

#[cfg(feature = "advanced-minification")]
use oxc_allocator::Allocator;
#[cfg(feature = "advanced-minification")]
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions};
#[cfg(feature = "advanced-minification")]
use oxc_mangler::MangleOptions as OxcMangleOptions;
#[cfg(feature = "advanced-minification")]
use oxc_minifier::{CompressOptions as OxcCompressOptions, Minifier, MinifierOptions};
#[cfg(feature = "advanced-minification")]
use oxc_parser::Parser;
#[cfg(feature = "advanced-minification")]
use oxc_span::SourceType;

/// Advanced minification configuration
#[derive(Debug, Clone)]
pub struct AdvancedMinifyConfig {
	/// Enable variable name mangling
	pub mangle: bool,
	/// Enable compression optimizations
	pub compress: bool,
	/// Remove console.* statements
	pub drop_console: bool,
	/// Remove debugger statements
	pub drop_debugger: bool,
	/// Mangle top-level variables
	pub toplevel: bool,
	/// Keep function names
	pub keep_fnames: bool,
	/// Keep class names
	pub keep_classnames: bool,
	/// Remove dead code
	pub dead_code_elimination: bool,
}

impl Default for AdvancedMinifyConfig {
	fn default() -> Self {
		Self {
			mangle: true,
			compress: true,
			drop_console: false,
			drop_debugger: true,
			toplevel: false,
			keep_fnames: false,
			keep_classnames: false,
			dead_code_elimination: true,
		}
	}
}

impl AdvancedMinifyConfig {
	/// Create a new configuration with default settings
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::AdvancedMinifyConfig;
	///
	/// let config = AdvancedMinifyConfig::new();
	/// assert!(config.mangle);
	/// assert!(config.compress);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Enable or disable variable mangling
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::AdvancedMinifyConfig;
	///
	/// let config = AdvancedMinifyConfig::new().with_mangle(true);
	/// assert!(config.mangle);
	/// ```
	pub fn with_mangle(mut self, enable: bool) -> Self {
		self.mangle = enable;
		self
	}

	/// Enable or disable compression
	pub fn with_compress(mut self, enable: bool) -> Self {
		self.compress = enable;
		self
	}

	/// Enable or disable console removal
	pub fn with_drop_console(mut self, enable: bool) -> Self {
		self.drop_console = enable;
		self
	}

	/// Enable or disable debugger removal
	pub fn with_drop_debugger(mut self, enable: bool) -> Self {
		self.drop_debugger = enable;
		self
	}

	/// Enable or disable top-level mangling
	pub fn with_toplevel(mut self, enable: bool) -> Self {
		self.toplevel = enable;
		self
	}

	/// Enable or disable function name preservation
	pub fn with_keep_fnames(mut self, enable: bool) -> Self {
		self.keep_fnames = enable;
		self
	}

	/// Enable or disable class name preservation
	pub fn with_keep_classnames(mut self, enable: bool) -> Self {
		self.keep_classnames = enable;
		self
	}

	/// Enable or disable dead code elimination
	pub fn with_dead_code_elimination(mut self, enable: bool) -> Self {
		self.dead_code_elimination = enable;
		self
	}

	/// Create a production configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::AdvancedMinifyConfig;
	///
	/// let config = AdvancedMinifyConfig::production();
	/// assert!(config.mangle);
	/// assert!(config.compress);
	/// assert!(config.drop_console);
	/// ```
	pub fn production() -> Self {
		Self {
			mangle: true,
			compress: true,
			drop_console: true,
			drop_debugger: true,
			toplevel: false,
			keep_fnames: false,
			keep_classnames: false,
			dead_code_elimination: true,
		}
	}

	/// Create a development configuration (minimal minification)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::AdvancedMinifyConfig;
	///
	/// let config = AdvancedMinifyConfig::development();
	/// assert!(!config.mangle);
	/// assert!(!config.drop_console);
	/// ```
	pub fn development() -> Self {
		Self {
			mangle: false,
			compress: false,
			drop_console: false,
			drop_debugger: false,
			toplevel: false,
			keep_fnames: true,
			keep_classnames: true,
			dead_code_elimination: false,
		}
	}
}

/// Advanced JavaScript minifier using OXC
///
/// This minifier provides production-grade optimization including:
/// - Variable name mangling for smaller output
/// - Dead code elimination
/// - Advanced compression techniques
/// - Optional console.log removal
pub struct AdvancedJsMinifier {
	config: AdvancedMinifyConfig,
}

impl AdvancedJsMinifier {
	/// Create a new advanced minifier with default settings
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::AdvancedJsMinifier;
	///
	/// let minifier = AdvancedJsMinifier::new();
	/// ```
	pub fn new() -> Self {
		Self {
			config: AdvancedMinifyConfig::default(),
		}
	}

	/// Create a minifier with custom configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::processing::advanced_minify::{AdvancedJsMinifier, AdvancedMinifyConfig};
	///
	/// let config = AdvancedMinifyConfig::production();
	/// let minifier = AdvancedJsMinifier::with_config(config);
	/// ```
	pub fn with_config(config: AdvancedMinifyConfig) -> Self {
		Self { config }
	}

	/// Create a production-ready minifier
	pub fn production() -> Self {
		Self {
			config: AdvancedMinifyConfig::production(),
		}
	}

	/// Create a development minifier
	pub fn development() -> Self {
		Self {
			config: AdvancedMinifyConfig::development(),
		}
	}

	/// Minify JavaScript code using OXC
	#[cfg(feature = "advanced-minification")]
	fn minify_with_oxc(&self, source: &str) -> ProcessingResult<String> {
		// Create allocator for OXC
		let allocator = Allocator::default();

		// Parse the source code - use script mode by default
		let source_type = SourceType::default();
		let parser = Parser::new(&allocator, source, source_type);
		let parse_result = parser.parse();

		if !parse_result.errors.is_empty() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				format!("Parse error: {:?}", parse_result.errors[0]),
			));
		}

		let mut program = parse_result.program;

		// Configure minifier options
		let mangle_options = if self.config.mangle {
			Some(OxcMangleOptions::default())
		} else {
			None
		};

		let compress_options = if self.config.compress {
			let mut opts = OxcCompressOptions::smallest();
			opts.drop_console = self.config.drop_console;
			opts.drop_debugger = self.config.drop_debugger;
			Some(opts)
		} else {
			None
		};

		let minifier_options = MinifierOptions {
			mangle: mangle_options,
			compress: compress_options,
		};

		// Run minifier
		let minifier_ret = Minifier::new(minifier_options).minify(&allocator, &mut program);

		// Generate code
		let codegen_options = CodegenOptions {
			minify: true,
			comments: CommentOptions::disabled(),
			..CodegenOptions::default()
		};

		let codegen_ret = Codegen::new()
			.with_options(codegen_options)
			.with_scoping(minifier_ret.scoping)
			.build(&program);

		Ok(codegen_ret.code)
	}

	/// Fallback minification when advanced-minification feature is disabled
	#[cfg(not(feature = "advanced-minification"))]
	fn minify_with_oxc(&self, source: &str) -> ProcessingResult<String> {
		Err(io::Error::new(
			io::ErrorKind::Unsupported,
			"Advanced minification requires the 'advanced-minification' feature flag",
		))
	}
}

impl Default for AdvancedJsMinifier {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Processor for AdvancedJsMinifier {
	async fn process(&self, input: &[u8], _path: &Path) -> ProcessingResult<Vec<u8>> {
		let source = String::from_utf8_lossy(input);
		let minified = self.minify_with_oxc(&source)?;
		Ok(minified.into_bytes())
	}

	fn can_process(&self, path: &Path) -> bool {
		path.extension()
			.and_then(|ext| ext.to_str())
			.map(|ext| {
				ext.eq_ignore_ascii_case("js")
					|| ext.eq_ignore_ascii_case("mjs")
					|| ext.eq_ignore_ascii_case("cjs")
			})
			.unwrap_or(false)
	}

	fn name(&self) -> &str {
		"AdvancedJsMinifier"
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_config_creation() {
		let config = AdvancedMinifyConfig::new();
		assert!(config.mangle);
		assert!(config.compress);
		assert!(!config.drop_console);
	}

	#[test]
	fn test_config_builder() {
		let config = AdvancedMinifyConfig::new()
			.with_mangle(true)
			.with_compress(true)
			.with_drop_console(true);

		assert!(config.mangle);
		assert!(config.compress);
		assert!(config.drop_console);
	}

	#[test]
	fn test_production_config() {
		let config = AdvancedMinifyConfig::production();
		assert!(config.mangle);
		assert!(config.compress);
		assert!(config.drop_console);
		assert!(config.drop_debugger);
	}

	#[test]
	fn test_development_config() {
		let config = AdvancedMinifyConfig::development();
		assert!(!config.mangle);
		assert!(!config.compress);
		assert!(!config.drop_console);
		assert!(config.keep_fnames);
	}

	#[test]
	fn test_minifier_creation() {
		let minifier = AdvancedJsMinifier::new();
		assert!(minifier.config.mangle);
	}

	#[test]
	fn test_minifier_with_config() {
		let config = AdvancedMinifyConfig::production();
		let minifier = AdvancedJsMinifier::with_config(config);
		assert!(minifier.config.drop_console);
	}

	#[test]
	fn test_minifier_production() {
		let minifier = AdvancedJsMinifier::production();
		assert!(minifier.config.mangle);
		assert!(minifier.config.compress);
	}

	#[test]
	fn test_minifier_development() {
		let minifier = AdvancedJsMinifier::development();
		assert!(!minifier.config.mangle);
		assert!(!minifier.config.compress);
	}

	#[test]
	fn test_can_process_js() {
		let minifier = AdvancedJsMinifier::new();
		assert!(minifier.can_process(&std::path::PathBuf::from("app.js")));
		assert!(minifier.can_process(&std::path::PathBuf::from("app.JS")));
	}

	#[test]
	fn test_can_process_mjs() {
		let minifier = AdvancedJsMinifier::new();
		assert!(minifier.can_process(&std::path::PathBuf::from("module.mjs")));
	}

	#[test]
	fn test_can_process_cjs() {
		let minifier = AdvancedJsMinifier::new();
		assert!(minifier.can_process(&std::path::PathBuf::from("common.cjs")));
	}

	#[test]
	fn test_cannot_process_other() {
		let minifier = AdvancedJsMinifier::new();
		assert!(!minifier.can_process(&std::path::PathBuf::from("style.css")));
		assert!(!minifier.can_process(&std::path::PathBuf::from("data.json")));
	}

	// Advanced minification tests (feature-gated)
	#[cfg(feature = "advanced-minification")]
	#[tokio::test]
	async fn test_minify_simple_code() {
		let minifier = AdvancedJsMinifier::new();
		// Use code that has side effects so it won't be eliminated
		let input = b"const x = 1; const y = 2; console.log(x + y);";
		let result = minifier
			.process(input, &std::path::PathBuf::from("test.js"))
			.await
			.unwrap();

		let output = String::from_utf8(result).unwrap();
		assert!(!output.is_empty());
		// With minification, should be smaller
		assert!(output.len() <= input.len());
		// Should still contain console.log since drop_console is false by default
		assert!(output.contains("console"));
	}

	#[cfg(feature = "advanced-minification")]
	#[tokio::test]
	async fn test_minify_with_console() {
		let minifier =
			AdvancedJsMinifier::with_config(AdvancedMinifyConfig::new().with_drop_console(true));
		let input = b"const x = 1; console.log(x); const y = 2;";
		let result = minifier
			.process(input, &std::path::PathBuf::from("test.js"))
			.await
			.unwrap();

		let output = String::from_utf8(result).unwrap();
		// console.log should be removed
		assert!(!output.contains("console"));
	}

	#[cfg(feature = "advanced-minification")]
	#[tokio::test]
	async fn test_minify_with_debugger() {
		let minifier =
			AdvancedJsMinifier::with_config(AdvancedMinifyConfig::new().with_drop_debugger(true));
		let input = b"const x = 1; debugger; const y = 2;";
		let result = minifier
			.process(input, &std::path::PathBuf::from("test.js"))
			.await
			.unwrap();

		let output = String::from_utf8(result).unwrap();
		// debugger should be removed
		assert!(!output.contains("debugger"));
	}

	#[cfg(feature = "advanced-minification")]
	#[tokio::test]
	async fn test_minify_production() {
		let minifier = AdvancedJsMinifier::production();
		// Use code with global side effects
		let input = b"window.result = (function() { const x = 1; const y = 2; return x + y; })(); console.log(window.result);";
		let result = minifier
			.process(input, &std::path::PathBuf::from("test.js"))
			.await
			.unwrap();

		let output = String::from_utf8(result).unwrap();
		assert!(!output.is_empty());
		// Production mode should drop console
		assert!(!output.contains("console"));
		// window.result assignment should remain (it's a side effect)
		assert!(output.contains("window"));
	}

	#[cfg(not(feature = "advanced-minification"))]
	#[tokio::test]
	async fn test_minify_without_feature() {
		let minifier = AdvancedJsMinifier::new();
		let input = b"const x = 1;";
		let result = minifier
			.process(input, &std::path::PathBuf::from("test.js"))
			.await;

		assert!(result.is_err());
		assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
	}
}
