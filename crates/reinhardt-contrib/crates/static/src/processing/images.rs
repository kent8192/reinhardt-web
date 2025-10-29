//! Image optimization for static assets
//!
//! Provides optimization for PNG, JPEG, and WebP images to reduce file size
//! while maintaining visual quality.

use super::{ProcessingResult, Processor};
use async_trait::async_trait;
use std::io;
use std::path::Path;

/// Image optimizer
///
/// Optimizes images to reduce file size while maintaining quality.
pub struct ImageOptimizer {
    /// Quality level (1-100)
    #[allow(dead_code)]
    quality: u8,
    /// Enable lossy compression
    #[allow(dead_code)]
    lossy: bool,
}

impl ImageOptimizer {
    /// Create a new image optimizer with default settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::processing::images::ImageOptimizer;
    ///
    /// let optimizer = ImageOptimizer::new(85);
    /// ```
    pub fn new(quality: u8) -> Self {
        Self {
            quality: quality.clamp(1, 100),
            lossy: true,
        }
    }

    /// Create an optimizer with custom settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::processing::images::ImageOptimizer;
    ///
    /// let optimizer = ImageOptimizer::with_settings(90, false);
    /// ```
    pub fn with_settings(quality: u8, lossy: bool) -> Self {
        Self {
            quality: quality.clamp(1, 100),
            lossy,
        }
    }

    /// Optimize PNG image
    fn optimize_png(&self, input: &[u8]) -> ProcessingResult<Vec<u8>> {
        // For now, return input unchanged
        // Real implementation would use oxipng or similar
        Ok(input.to_vec())
    }

    /// Optimize JPEG image
    fn optimize_jpeg(&self, input: &[u8]) -> ProcessingResult<Vec<u8>> {
        // For now, return input unchanged
        // Real implementation would use mozjpeg or similar
        Ok(input.to_vec())
    }

    /// Optimize WebP image
    fn optimize_webp(&self, input: &[u8]) -> ProcessingResult<Vec<u8>> {
        // For now, return input unchanged
        // Real implementation would use libwebp
        Ok(input.to_vec())
    }

    /// Detect image format from file extension
    fn detect_format(&self, path: &Path) -> Option<ImageFormat> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "png" => Some(ImageFormat::Png),
                "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
                "webp" => Some(ImageFormat::WebP),
                _ => None,
            })
    }
}

impl Default for ImageOptimizer {
    fn default() -> Self {
        Self::new(85)
    }
}

#[async_trait]
impl Processor for ImageOptimizer {
    async fn process(&self, input: &[u8], path: &Path) -> ProcessingResult<Vec<u8>> {
        match self.detect_format(path) {
            Some(ImageFormat::Png) => self.optimize_png(input),
            Some(ImageFormat::Jpeg) => self.optimize_jpeg(input),
            Some(ImageFormat::WebP) => self.optimize_webp(input),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported image format",
            )),
        }
    }

    fn can_process(&self, path: &Path) -> bool {
        self.detect_format(path).is_some()
    }

    fn name(&self) -> &str {
        "ImageOptimizer"
    }
}

/// Image format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageFormat {
    Png,
    Jpeg,
    WebP,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = ImageOptimizer::new(85);
        assert_eq!(optimizer.quality, 85);
        assert!(optimizer.lossy);
    }

    #[test]
    fn test_optimizer_with_settings() {
        let optimizer = ImageOptimizer::with_settings(90, false);
        assert_eq!(optimizer.quality, 90);
        assert!(!optimizer.lossy);
    }

    #[test]
    fn test_quality_clamping() {
        let optimizer1 = ImageOptimizer::new(150);
        assert_eq!(optimizer1.quality, 100);

        let optimizer2 = ImageOptimizer::new(0);
        assert_eq!(optimizer2.quality, 1);
    }

    #[test]
    fn test_can_process_png() {
        let optimizer = ImageOptimizer::new(85);
        assert!(optimizer.can_process(&PathBuf::from("image.png")));
        assert!(optimizer.can_process(&PathBuf::from("image.PNG")));
    }

    #[test]
    fn test_can_process_jpeg() {
        let optimizer = ImageOptimizer::new(85);
        assert!(optimizer.can_process(&PathBuf::from("photo.jpg")));
        assert!(optimizer.can_process(&PathBuf::from("photo.jpeg")));
        assert!(optimizer.can_process(&PathBuf::from("photo.JPEG")));
    }

    #[test]
    fn test_can_process_webp() {
        let optimizer = ImageOptimizer::new(85);
        assert!(optimizer.can_process(&PathBuf::from("image.webp")));
        assert!(optimizer.can_process(&PathBuf::from("image.WEBP")));
    }

    #[test]
    fn test_cannot_process_other_formats() {
        let optimizer = ImageOptimizer::new(85);
        assert!(!optimizer.can_process(&PathBuf::from("style.css")));
        assert!(!optimizer.can_process(&PathBuf::from("script.js")));
        assert!(!optimizer.can_process(&PathBuf::from("image.gif")));
    }

    #[tokio::test]
    async fn test_optimize_png_basic() {
        let optimizer = ImageOptimizer::new(85);
        let input = b"fake png data";
        let result = optimizer
            .process(input, &PathBuf::from("test.png"))
            .await
            .unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_optimize_jpeg_basic() {
        let optimizer = ImageOptimizer::new(85);
        let input = b"fake jpeg data";
        let result = optimizer
            .process(input, &PathBuf::from("test.jpg"))
            .await
            .unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_optimize_webp_basic() {
        let optimizer = ImageOptimizer::new(85);
        let input = b"fake webp data";
        let result = optimizer
            .process(input, &PathBuf::from("test.webp"))
            .await
            .unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_unsupported_format() {
        let optimizer = ImageOptimizer::new(85);
        let input = b"data";
        let result = optimizer.process(input, &PathBuf::from("test.gif")).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_default() {
        let optimizer = ImageOptimizer::default();
        assert_eq!(optimizer.quality, 85);
        assert!(optimizer.lossy);
    }
}
