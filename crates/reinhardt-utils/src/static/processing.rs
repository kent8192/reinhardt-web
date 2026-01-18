//! File processing pipeline for static assets
//!
//! This module provides a comprehensive file processing pipeline for optimizing
//! and transforming static assets before deployment. It includes:
//!
//! - CSS/JavaScript minification
//! - Image optimization (PNG, JPEG, WebP)
//! - Source map generation
//! - Asset bundling and concatenation
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt_utils::r#static::processing::{ProcessingPipeline, ProcessingConfig};
//!
//! let config = ProcessingConfig::default()
//!     .with_minification(true)
//!     .with_source_maps(true);
//!
//! let pipeline = ProcessingPipeline::new(config);
//! // let result = pipeline.process_file("app.js").await?;
//! ```

use async_trait::async_trait;
use std::io;
use std::path::{Path, PathBuf};

pub mod bundle;
pub mod compress;
pub mod minify;

#[cfg(feature = "advanced-minification")]
pub mod advanced_minify;

#[cfg(feature = "image-optimization")]
pub mod images;

#[cfg(feature = "source-maps")]
pub mod sourcemap;

/// Result type for processing operations
pub type ProcessingResult<T> = io::Result<T>;

/// Trait for file processors
///
/// All processors must implement this trait to be used in the processing pipeline.
#[async_trait]
pub trait Processor: Send + Sync {
	/// Process a file and return the processed content
	///
	/// # Arguments
	///
	/// * `input` - The input file content
	/// * `path` - The original file path (for context)
	///
	/// # Returns
	///
	/// The processed file content
	async fn process(&self, input: &[u8], path: &Path) -> ProcessingResult<Vec<u8>>;

	/// Check if this processor can handle the given file
	///
	/// # Arguments
	///
	/// * `path` - The file path to check
	///
	/// # Returns
	///
	/// `true` if this processor can handle the file, `false` otherwise
	fn can_process(&self, path: &Path) -> bool;

	/// Get the processor name for logging/debugging
	fn name(&self) -> &str;
}

/// Configuration for the processing pipeline
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
	/// Enable CSS/JS minification
	pub minify: bool,
	/// Enable source map generation
	pub source_maps: bool,
	/// Enable image optimization
	pub optimize_images: bool,
	/// Enable asset bundling
	pub bundle: bool,
	/// Output directory for processed files
	pub output_dir: PathBuf,
	/// Compression level for images (1-100)
	pub image_quality: u8,
}

impl Default for ProcessingConfig {
	fn default() -> Self {
		Self {
			minify: true,
			source_maps: false,
			optimize_images: true,
			bundle: false,
			output_dir: PathBuf::from("static/processed"),
			image_quality: 85,
		}
	}
}

impl ProcessingConfig {
	/// Create a new processing configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::ProcessingConfig;
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"));
	/// assert!(config.minify);
	/// assert!(config.optimize_images);
	/// ```
	pub fn new(output_dir: PathBuf) -> Self {
		Self {
			output_dir,
			..Default::default()
		}
	}

	/// Enable or disable minification
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::ProcessingConfig;
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"))
	///     .with_minification(true);
	/// assert!(config.minify);
	/// ```
	pub fn with_minification(mut self, enable: bool) -> Self {
		self.minify = enable;
		self
	}

	/// Enable or disable source maps
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::ProcessingConfig;
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"))
	///     .with_source_maps(true);
	/// assert!(config.source_maps);
	/// ```
	pub fn with_source_maps(mut self, enable: bool) -> Self {
		self.source_maps = enable;
		self
	}

	/// Enable or disable image optimization
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::ProcessingConfig;
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"))
	///     .with_image_optimization(false);
	/// assert!(!config.optimize_images);
	/// ```
	pub fn with_image_optimization(mut self, enable: bool) -> Self {
		self.optimize_images = enable;
		self
	}

	/// Set image quality (1-100)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::ProcessingConfig;
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"))
	///     .with_image_quality(90);
	/// assert_eq!(config.image_quality, 90);
	/// ```
	pub fn with_image_quality(mut self, quality: u8) -> Self {
		self.image_quality = quality.clamp(1, 100);
		self
	}
}

/// Processing pipeline for static assets
///
/// Manages the processing of static files through multiple processors.
pub struct ProcessingPipeline {
	config: ProcessingConfig,
	processors: Vec<Box<dyn Processor>>,
}

impl ProcessingPipeline {
	/// Create a new processing pipeline
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::{ProcessingPipeline, ProcessingConfig};
	/// use std::path::PathBuf;
	///
	/// let config = ProcessingConfig::new(PathBuf::from("dist"));
	/// let pipeline = ProcessingPipeline::new(config);
	/// ```
	pub fn new(config: ProcessingConfig) -> Self {
		let mut processors: Vec<Box<dyn Processor>> = Vec::new();

		// Add minifier if enabled
		if config.minify {
			processors.push(Box::new(minify::CssMinifier::new()));
			processors.push(Box::new(minify::JsMinifier::new()));
		}

		// Add image optimizer if enabled
		#[cfg(feature = "image-optimization")]
		if config.optimize_images {
			processors.push(Box::new(images::ImageOptimizer::new(config.image_quality)));
		}

		Self { config, processors }
	}

	/// Process a single file
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_utils::r#static::processing::{ProcessingPipeline, ProcessingConfig};
	/// use std::path::PathBuf;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let config = ProcessingConfig::new(PathBuf::from("dist"));
	/// let pipeline = ProcessingPipeline::new(config);
	///
	/// let content = b"body { color: red; }";
	/// let processed = pipeline.process_file(content, &PathBuf::from("style.css")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn process_file(&self, input: &[u8], path: &Path) -> ProcessingResult<Vec<u8>> {
		let mut content = input.to_vec();

		for processor in &self.processors {
			if processor.can_process(path) {
				content = processor.process(&content, path).await?;
			}
		}

		Ok(content)
	}

	/// Get the configuration
	pub fn config(&self) -> &ProcessingConfig {
		&self.config
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_processing_config_default() {
		let config = ProcessingConfig::default();
		assert!(config.minify);
		assert!(!config.source_maps);
		assert!(config.optimize_images);
		assert!(!config.bundle);
		assert_eq!(config.image_quality, 85);
	}

	#[test]
	fn test_processing_config_builder() {
		let config = ProcessingConfig::new(PathBuf::from("dist"))
			.with_minification(false)
			.with_source_maps(true)
			.with_image_optimization(false)
			.with_image_quality(90);

		assert!(!config.minify);
		assert!(config.source_maps);
		assert!(!config.optimize_images);
		assert_eq!(config.image_quality, 90);
	}

	#[test]
	fn test_image_quality_clamping() {
		let config1 = ProcessingConfig::default().with_image_quality(150);
		assert_eq!(config1.image_quality, 100);

		let config2 = ProcessingConfig::default().with_image_quality(0);
		assert_eq!(config2.image_quality, 1);
	}

	#[test]
	fn test_pipeline_creation() {
		let config = ProcessingConfig::new(PathBuf::from("dist"));
		let pipeline = ProcessingPipeline::new(config);
		assert!(pipeline.config().minify);
	}

	#[tokio::test]
	async fn test_pipeline_process_empty() {
		let config = ProcessingConfig::new(PathBuf::from("dist"));
		let pipeline = ProcessingPipeline::new(config);

		let result = pipeline
			.process_file(b"test", &PathBuf::from("test.txt"))
			.await
			.unwrap();
		assert_eq!(result, b"test");
	}
}
