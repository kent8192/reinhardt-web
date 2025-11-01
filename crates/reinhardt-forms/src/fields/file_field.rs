use crate::field::{FieldError, FieldResult, FormField, Widget};

/// FileField for file upload
pub struct FileField {
	pub name: String,
	pub label: Option<String>,
	pub required: bool,
	pub help_text: Option<String>,
	pub widget: Widget,
	pub initial: Option<serde_json::Value>,
	pub max_length: Option<usize>,
	pub allow_empty_file: bool,
}

impl FileField {
	/// Create a new FileField
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::FileField;
	///
	/// let field = FileField::new("upload".to_string());
	/// assert_eq!(field.name, "upload");
	/// ```
	pub fn new(name: String) -> Self {
		Self {
			name,
			label: None,
			required: true,
			help_text: None,
			widget: Widget::FileInput,
			initial: None,
			max_length: None,
			allow_empty_file: false,
		}
	}
}

impl FormField for FileField {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> Option<&str> {
		self.label.as_deref()
	}

	fn required(&self) -> bool {
		self.required
	}

	fn help_text(&self) -> Option<&str> {
		self.help_text.as_deref()
	}

	fn widget(&self) -> &Widget {
		&self.widget
	}

	fn initial(&self) -> Option<&serde_json::Value> {
		self.initial.as_ref()
	}

	fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
		match value {
			None if self.required => Err(FieldError::required(None)),
			None => Ok(serde_json::Value::Null),
			Some(v) => {
				// Expect an object with filename and optional size
				let obj = v
					.as_object()
					.ok_or_else(|| FieldError::Invalid("Expected object".to_string()))?;

				let filename = obj
					.get("filename")
					.and_then(|f| f.as_str())
					.ok_or_else(|| FieldError::Invalid("Missing filename".to_string()))?;

				if filename.is_empty() {
					if self.required {
						return Err(FieldError::required(None));
					}
					return Ok(serde_json::Value::Null);
				}

				// Check filename length
				if let Some(max) = self.max_length
					&& filename.len() > max {
						return Err(FieldError::Validation(format!(
							"Filename is too long (max {} characters)",
							max
						)));
					}

				// Check for empty file
				if !self.allow_empty_file
					&& let Some(size) = obj.get("size").and_then(|s| s.as_u64())
						&& size == 0 {
							return Err(FieldError::Validation(
								"The submitted file is empty".to_string(),
							));
						}

				Ok(v.clone())
			}
		}
	}
}

/// ImageField for image upload with additional validation
pub struct ImageField {
	pub name: String,
	pub label: Option<String>,
	pub required: bool,
	pub help_text: Option<String>,
	pub widget: Widget,
	pub initial: Option<serde_json::Value>,
	pub max_length: Option<usize>,
	pub allow_empty_file: bool,
}

impl ImageField {
	pub fn new(name: String) -> Self {
		Self {
			name,
			label: None,
			required: true,
			help_text: None,
			widget: Widget::FileInput,
			initial: None,
			max_length: None,
			allow_empty_file: false,
		}
	}

	fn is_valid_image_extension(filename: &str) -> bool {
		let valid_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"];
		filename
			.rsplit('.')
			.next()
			.map(|ext| valid_extensions.contains(&ext.to_lowercase().as_str()))
			.unwrap_or(false)
	}
}

impl FormField for ImageField {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> Option<&str> {
		self.label.as_deref()
	}

	fn required(&self) -> bool {
		self.required
	}

	fn help_text(&self) -> Option<&str> {
		self.help_text.as_deref()
	}

	fn widget(&self) -> &Widget {
		&self.widget
	}

	fn initial(&self) -> Option<&serde_json::Value> {
		self.initial.as_ref()
	}

	fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
		match value {
			None if self.required => Err(FieldError::required(None)),
			None => Ok(serde_json::Value::Null),
			Some(v) => {
				let obj = v
					.as_object()
					.ok_or_else(|| FieldError::Invalid("Expected object".to_string()))?;

				let filename = obj
					.get("filename")
					.and_then(|f| f.as_str())
					.ok_or_else(|| FieldError::Invalid("Missing filename".to_string()))?;

				if filename.is_empty() {
					if self.required {
						return Err(FieldError::required(None));
					}
					return Ok(serde_json::Value::Null);
				}

				// Validate image extension
				if !Self::is_valid_image_extension(filename) {
					return Err(FieldError::Validation(
                        "Upload a valid image. The file you uploaded was either not an image or a corrupted image".to_string(),
                    ));
				}

				// Check filename length
				if let Some(max) = self.max_length
					&& filename.len() > max {
						return Err(FieldError::Validation(format!(
							"Filename is too long (max {} characters)",
							max
						)));
					}

				// Check for empty file
				if !self.allow_empty_file
					&& let Some(size) = obj.get("size").and_then(|s| s.as_u64())
						&& size == 0 {
							return Err(FieldError::Validation(
								"The submitted file is empty".to_string(),
							));
						}

				Ok(v.clone())
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_filefield_valid() {
		let field = FileField::new("document".to_string());

		let file = serde_json::json!({
			"filename": "test.pdf",
			"size": 1024
		});

		assert!(field.clean(Some(&file)).is_ok());
	}

	#[test]
	fn test_filefield_empty() {
		let field = FileField::new("document".to_string());

		let file = serde_json::json!({
			"filename": "test.pdf",
			"size": 0
		});

		assert!(matches!(
			field.clean(Some(&file)),
			Err(FieldError::Validation(_))
		));
	}

	#[test]
	fn test_imagefield_valid() {
		let field = ImageField::new("photo".to_string());

		let file = serde_json::json!({
			"filename": "test.jpg",
			"size": 1024
		});

		assert!(field.clean(Some(&file)).is_ok());
	}

	#[test]
	fn test_imagefield_invalid_extension() {
		let field = ImageField::new("photo".to_string());

		let file = serde_json::json!({
			"filename": "test.pdf",
			"size": 1024
		});

		assert!(matches!(
			field.clean(Some(&file)),
			Err(FieldError::Validation(_))
		));
	}
}
