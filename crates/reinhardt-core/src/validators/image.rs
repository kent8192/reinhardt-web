//! Image dimension validator for validating image width, height, and aspect ratio
//!
//! This validator provides validation for image dimensions with support for:
//! - Min/max width constraints
//! - Min/max height constraints
//! - Aspect ratio validation with tolerance
//! - Multiple image format support (JPEG, PNG, GIF, WebP, etc.)
//!
//! # Examples
//!
//! ## Validate minimum dimensions
//!
//! ```
//! use reinhardt_core::validators::ImageDimensionValidator;
//!
//! let validator = ImageDimensionValidator::new()
//!     .with_min_width(800)
//!     .with_min_height(600);
//!
//! // Validate from bytes (e.g., uploaded file)
//! // let result = validator.validate_bytes(&image_data);
//! ```
//!
//! ## Validate aspect ratio
//!
//! ```
//! use reinhardt_core::validators::ImageDimensionValidator;
//!
//! let validator = ImageDimensionValidator::new()
//!     .with_aspect_ratio(16, 9)
//!     .with_aspect_ratio_tolerance(0.01); // 1% tolerance
//!
//! // This will validate that the image has 16:9 aspect ratio ± 1%
//! ```

use std::path::Path;

use image::GenericImageView;

use super::{ValidationError, ValidationResult};

/// Image dimension validator with width, height, and aspect ratio constraints
///
/// Validates image dimensions using the `image` crate.
/// Supports all formats that the `image` crate supports (JPEG, PNG, GIF, WebP, etc.)
pub struct ImageDimensionValidator {
	min_width: Option<u32>,
	max_width: Option<u32>,
	min_height: Option<u32>,
	max_height: Option<u32>,
	aspect_ratio: Option<(u32, u32)>,
	aspect_ratio_tolerance: f32,
}

impl ImageDimensionValidator {
	/// Create a new image dimension validator with no constraints
	pub fn new() -> Self {
		Self {
			min_width: None,
			max_width: None,
			min_height: None,
			max_height: None,
			aspect_ratio: None,
			aspect_ratio_tolerance: 0.0,
		}
	}

	/// Set minimum width constraint
	pub fn with_min_width(mut self, width: u32) -> Self {
		self.min_width = Some(width);
		self
	}

	/// Set maximum width constraint
	pub fn with_max_width(mut self, width: u32) -> Self {
		self.max_width = Some(width);
		self
	}

	/// Set width range constraint (both min and max)
	pub fn with_width_range(mut self, min: u32, max: u32) -> Self {
		self.min_width = Some(min);
		self.max_width = Some(max);
		self
	}

	/// Set minimum height constraint
	pub fn with_min_height(mut self, height: u32) -> Self {
		self.min_height = Some(height);
		self
	}

	/// Set maximum height constraint
	pub fn with_max_height(mut self, height: u32) -> Self {
		self.max_height = Some(height);
		self
	}

	/// Set height range constraint (both min and max)
	pub fn with_height_range(mut self, min: u32, max: u32) -> Self {
		self.min_height = Some(min);
		self.max_height = Some(max);
		self
	}

	/// Set aspect ratio constraint (width:height)
	///
	/// # Example
	///
	/// ```
	/// use reinhardt_core::validators::ImageDimensionValidator;
	///
	/// // 16:9 aspect ratio
	/// let validator = ImageDimensionValidator::new()
	///     .with_aspect_ratio(16, 9);
	///
	/// // 4:3 aspect ratio
	/// let validator = ImageDimensionValidator::new()
	///     .with_aspect_ratio(4, 3);
	/// ```
	pub fn with_aspect_ratio(mut self, width: u32, height: u32) -> Self {
		self.aspect_ratio = Some((width, height));
		self
	}

	/// Set aspect ratio tolerance (0.0 to 1.0)
	///
	/// Default is 0.0 (exact match required).
	/// For example, 0.01 means 1% tolerance.
	///
	/// # Example
	///
	/// ```
	/// use reinhardt_core::validators::ImageDimensionValidator;
	///
	/// let validator = ImageDimensionValidator::new()
	///     .with_aspect_ratio(16, 9)
	///     .with_aspect_ratio_tolerance(0.05); // 5% tolerance
	/// ```
	pub fn with_aspect_ratio_tolerance(mut self, tolerance: f32) -> Self {
		self.aspect_ratio_tolerance = tolerance;
		self
	}

	/// Validate image dimensions from a file path
	///
	/// # Errors
	///
	/// Returns error if:
	/// - File cannot be read
	/// - Image format is not supported
	/// - Dimensions don't meet constraints
	pub fn validate_file(&self, path: impl AsRef<Path>) -> ValidationResult<()> {
		let img = image::open(path).map_err(|e| ValidationError::ImageReadError(e.to_string()))?;
		let (width, height) = img.dimensions();
		self.validate_dimensions(width, height)
	}

	/// Validate image dimensions from byte data
	///
	/// # Errors
	///
	/// Returns error if:
	/// - Image data is invalid
	/// - Image format is not supported
	/// - Dimensions don't meet constraints
	pub fn validate_bytes(&self, data: &[u8]) -> ValidationResult<()> {
		let img = image::load_from_memory(data)
			.map_err(|e| ValidationError::ImageReadError(e.to_string()))?;
		let (width, height) = img.dimensions();
		self.validate_dimensions(width, height)
	}

	/// Validate image dimensions
	fn validate_dimensions(&self, width: u32, height: u32) -> ValidationResult<()> {
		// Validate width
		if let Some(min_width) = self.min_width
			&& width < min_width
		{
			return Err(ValidationError::ImageWidthTooSmall { width, min_width });
		}

		if let Some(max_width) = self.max_width
			&& width > max_width
		{
			return Err(ValidationError::ImageWidthTooLarge { width, max_width });
		}

		// Validate height
		if let Some(min_height) = self.min_height
			&& height < min_height
		{
			return Err(ValidationError::ImageHeightTooSmall { height, min_height });
		}

		if let Some(max_height) = self.max_height
			&& height > max_height
		{
			return Err(ValidationError::ImageHeightTooLarge { height, max_height });
		}

		// Validate aspect ratio
		if let Some((expected_width, expected_height)) = self.aspect_ratio {
			let actual_ratio = width as f32 / height as f32;
			let expected_ratio = expected_width as f32 / expected_height as f32;
			let diff = (actual_ratio - expected_ratio).abs();
			let tolerance = expected_ratio * self.aspect_ratio_tolerance;

			if diff > tolerance {
				return Err(ValidationError::InvalidAspectRatio {
					actual_width: width,
					actual_height: height,
					expected_width,
					expected_height,
				});
			}
		}

		Ok(())
	}
}

impl Default for ImageDimensionValidator {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// Helper function to create a test image in memory
	fn create_test_image(width: u32, height: u32) -> Vec<u8> {
		use image::{ImageBuffer, Rgb};

		let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
		let mut bytes = Vec::new();
		img.write_to(
			&mut std::io::Cursor::new(&mut bytes),
			image::ImageFormat::Png,
		)
		.unwrap();
		bytes
	}

	// Width validation tests
	#[rstest]
	fn test_min_width_pass() {
		let validator = ImageDimensionValidator::new().with_min_width(800);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_min_width_fail() {
		let validator = ImageDimensionValidator::new().with_min_width(1920);
		let image_data = create_test_image(1024, 768);
		match validator.validate_bytes(&image_data) {
			Err(ValidationError::ImageWidthTooSmall { width, min_width }) => {
				assert_eq!(width, 1024);
				assert_eq!(min_width, 1920);
			}
			_ => panic!("Expected ImageWidthTooSmall error"),
		}
	}

	#[rstest]
	fn test_max_width_pass() {
		let validator = ImageDimensionValidator::new().with_max_width(2048);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_max_width_fail() {
		let validator = ImageDimensionValidator::new().with_max_width(800);
		let image_data = create_test_image(1024, 768);
		match validator.validate_bytes(&image_data) {
			Err(ValidationError::ImageWidthTooLarge { width, max_width }) => {
				assert_eq!(width, 1024);
				assert_eq!(max_width, 800);
			}
			_ => panic!("Expected ImageWidthTooLarge error"),
		}
	}

	#[rstest]
	fn test_width_range_pass() {
		let validator = ImageDimensionValidator::new().with_width_range(800, 2048);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	// Height validation tests
	#[rstest]
	fn test_min_height_pass() {
		let validator = ImageDimensionValidator::new().with_min_height(600);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_min_height_fail() {
		let validator = ImageDimensionValidator::new().with_min_height(1080);
		let image_data = create_test_image(1024, 768);
		match validator.validate_bytes(&image_data) {
			Err(ValidationError::ImageHeightTooSmall { height, min_height }) => {
				assert_eq!(height, 768);
				assert_eq!(min_height, 1080);
			}
			_ => panic!("Expected ImageHeightTooSmall error"),
		}
	}

	#[rstest]
	fn test_max_height_pass() {
		let validator = ImageDimensionValidator::new().with_max_height(1080);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_max_height_fail() {
		let validator = ImageDimensionValidator::new().with_max_height(600);
		let image_data = create_test_image(1024, 768);
		match validator.validate_bytes(&image_data) {
			Err(ValidationError::ImageHeightTooLarge { height, max_height }) => {
				assert_eq!(height, 768);
				assert_eq!(max_height, 600);
			}
			_ => panic!("Expected ImageHeightTooLarge error"),
		}
	}

	#[rstest]
	fn test_height_range_pass() {
		let validator = ImageDimensionValidator::new().with_height_range(600, 1080);
		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	// Aspect ratio tests
	#[rstest]
	fn test_aspect_ratio_exact_match() {
		let validator = ImageDimensionValidator::new().with_aspect_ratio(4, 3);
		let image_data = create_test_image(1024, 768); // 4:3
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_aspect_ratio_mismatch() {
		let validator = ImageDimensionValidator::new().with_aspect_ratio(16, 9);
		let image_data = create_test_image(1024, 768); // 4:3
		match validator.validate_bytes(&image_data) {
			Err(ValidationError::InvalidAspectRatio {
				actual_width,
				actual_height,
				expected_width,
				expected_height,
			}) => {
				assert_eq!(actual_width, 1024);
				assert_eq!(actual_height, 768);
				assert_eq!(expected_width, 16);
				assert_eq!(expected_height, 9);
			}
			_ => panic!("Expected InvalidAspectRatio error"),
		}
	}

	#[rstest]
	fn test_aspect_ratio_with_tolerance() {
		// 16:9 = 1.7777...
		// 1920x1080 = 1.7777... (exact)
		// 1920x1081 = 1.7761... (within 0.01 tolerance)
		let validator = ImageDimensionValidator::new()
			.with_aspect_ratio(16, 9)
			.with_aspect_ratio_tolerance(0.01); // 1% tolerance

		let image_data = create_test_image(1920, 1081);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	// Combined constraints tests
	#[rstest]
	fn test_combined_constraints_pass() {
		let validator = ImageDimensionValidator::new()
			.with_width_range(800, 2048)
			.with_height_range(600, 1536)
			.with_aspect_ratio(4, 3);

		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_combined_constraints_fail_width() {
		let validator = ImageDimensionValidator::new()
			.with_width_range(1200, 2048)
			.with_height_range(600, 1536);

		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_err());
	}

	// Invalid image data test
	#[rstest]
	fn test_invalid_image_data() {
		let validator = ImageDimensionValidator::new();
		let invalid_data = vec![0u8; 100]; // Not a valid image
		match validator.validate_bytes(&invalid_data) {
			Err(ValidationError::ImageReadError(_)) => {}
			_ => panic!("Expected ImageReadError"),
		}
	}

	// Edge cases
	#[rstest]
	fn test_no_constraints() {
		let validator = ImageDimensionValidator::new();
		let image_data = create_test_image(100, 100);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}

	#[rstest]
	fn test_exact_boundary() {
		let validator = ImageDimensionValidator::new()
			.with_min_width(1024)
			.with_max_width(1024);

		let image_data = create_test_image(1024, 768);
		assert!(validator.validate_bytes(&image_data).is_ok());
	}
}
