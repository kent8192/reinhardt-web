// File and Image fields for Django-style file uploads
// Corresponds to Django's FileField and ImageField

use super::fields::{BaseField, Field, FieldDeconstruction, FieldKwarg};
use cap_std::{ambient_authority, fs::Dir};
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

fn validate_upload_path(upload_to: &str, file_name: &str) -> Result<(), FileFieldError> {
	let directory = Path::new(upload_to);
	if directory
		.components()
		.any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
		|| upload_to.contains(['\\', ':', '\0'])
	{
		return Err(FileFieldError::InvalidPath(upload_to.into()));
	}

	let name = Path::new(file_name);
	if name.components().count() != 1
		|| !matches!(name.components().next(), Some(Component::Normal(_)))
		|| file_name.contains(['/', '\\', ':', '\0'])
	{
		return Err(FileFieldError::InvalidPath(file_name.into()));
	}

	Ok(())
}

/// Error types for file field operations
#[non_exhaustive]
#[derive(Debug)]
pub enum FileFieldError {
	/// IoError variant.
	IoError(io::Error),
	/// InvalidPath variant.
	InvalidPath(String),
	/// InvalidImage variant.
	InvalidImage(String),
	/// InvalidDimensions variant.
	InvalidDimensions {
		/// The expected.
		expected: (u32, u32),
		/// The actual.
		actual: (u32, u32),
	},
}

impl std::fmt::Display for FileFieldError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FileFieldError::IoError(e) => write!(f, "IO error: {}", e),
			FileFieldError::InvalidPath(p) => write!(f, "Invalid path: {}", p),
			FileFieldError::InvalidImage(msg) => write!(f, "Invalid image: {}", msg),
			FileFieldError::InvalidDimensions { expected, actual } => write!(
				f,
				"Invalid dimensions: expected {}x{}, got {}x{}",
				expected.0, expected.1, actual.0, actual.1
			),
		}
	}
}

impl std::error::Error for FileFieldError {}

impl From<io::Error> for FileFieldError {
	fn from(error: io::Error) -> Self {
		FileFieldError::IoError(error)
	}
}

/// Validates a path that is interpreted relative to the storage root.
///
/// Empty paths, absolute paths, platform prefixes, and `..` components are
/// rejected because joining them onto the storage root could leave it.
fn confined_relative_path(path: &Path) -> Result<&Path, FileFieldError> {
	let mut has_name = false;
	for component in path.components() {
		match component {
			Component::Normal(_) => has_name = true,
			Component::CurDir => {}
			Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
				return Err(FileFieldError::InvalidPath(
					path.to_string_lossy().into_owned(),
				));
			}
		}
	}
	if !has_name {
		return Err(FileFieldError::InvalidPath(
			path.to_string_lossy().into_owned(),
		));
	}
	Ok(path)
}

/// Opens the storage root as a directory capability.
///
/// Operations performed through the returned [`Dir`] resolve paths relative to
/// the root and fail instead of following `..` or symlinks out of it.
fn open_storage_root(storage_path: Option<&Path>, create: bool) -> io::Result<Dir> {
	let root = storage_path.unwrap_or_else(|| Path::new("."));
	if create {
		fs::create_dir_all(root)?;
	}
	Dir::open_ambient_dir(root, ambient_authority())
}

/// Writes `content` to `upload_to/file_name` below the storage root and
/// returns the root-relative path.
fn save_confined(
	storage_path: Option<&Path>,
	upload_to: &str,
	file_name: &str,
	content: &[u8],
) -> Result<String, FileFieldError> {
	confined_relative_path(Path::new(file_name))?;
	let joined = Path::new(upload_to).join(file_name);
	let relative_path = confined_relative_path(&joined)?;

	let root = open_storage_root(storage_path, true)?;
	if let Some(parent) = relative_path.parent()
		&& !parent.as_os_str().is_empty()
	{
		root.create_dir_all(parent)?;
	}
	let mut file = root.create(relative_path)?;
	file.write_all(content)?;

	Ok(relative_path.to_string_lossy().into_owned())
}

/// Reports whether a root-relative path exists without leaving the storage root.
fn exists_confined(storage_path: Option<&Path>, path: &str) -> bool {
	let Ok(relative_path) = confined_relative_path(Path::new(path)) else {
		return false;
	};
	open_storage_root(storage_path, false)
		.and_then(|root| root.metadata(relative_path))
		.is_ok()
}

/// Removes a root-relative file without leaving the storage root.
///
/// A missing storage root or file is treated as already deleted.
fn delete_confined(storage_path: Option<&Path>, path: &str) -> Result<(), FileFieldError> {
	let relative_path = confined_relative_path(Path::new(path))?;
	let root = match open_storage_root(storage_path, false) {
		Ok(root) => root,
		Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
		Err(error) => return Err(error.into()),
	};
	match root.remove_file(relative_path) {
		Ok(()) => Ok(()),
		Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
		Err(error) => Err(error.into()),
	}
}

/// FileField - handles file uploads and storage
///
/// # Examples
///
/// ```
/// use reinhardt_db::orm::file_fields::FileField;
///
/// let field = FileField::new("uploads/documents");
/// assert_eq!(field.upload_to, "uploads/documents");
/// assert_eq!(field.max_length, 100);
/// ```
#[derive(Debug, Clone)]
pub struct FileField {
	/// The base.
	pub base: BaseField,
	/// The upload to.
	pub upload_to: String,
	/// The max length.
	pub max_length: u64,
	/// The storage path.
	pub storage_path: Option<PathBuf>,
}

impl FileField {
	/// Create a new FileField
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::new("uploads/files");
	/// assert_eq!(field.upload_to, "uploads/files");
	/// assert_eq!(field.max_length, 100);
	/// ```
	pub fn new(upload_to: impl Into<String>) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length: 100,
			storage_path: None,
		}
	}

	/// Create a FileField with custom max_length
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::with_max_length("uploads/files", 255);
	/// assert_eq!(field.max_length, 255);
	/// ```
	pub fn with_max_length(upload_to: impl Into<String>, max_length: u64) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length,
			storage_path: None,
		}
	}

	/// Create a FileField with custom storage path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::FileField;
	/// use std::path::PathBuf;
	///
	/// let field = FileField::with_storage("uploads/files", PathBuf::from("/var/www/media"));
	/// assert_eq!(field.storage_path, Some(PathBuf::from("/var/www/media")));
	/// ```
	pub fn with_storage(upload_to: impl Into<String>, storage_path: PathBuf) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length: 100,
			storage_path: Some(storage_path),
		}
	}

	/// Save file content to storage
	///
	/// The file is written to `upload_to/file_name` below the storage root
	/// (`storage_path`, or the current directory when unset).
	///
	/// # Errors
	///
	/// Returns [`FileFieldError::InvalidPath`] when `file_name` is not a
	/// single plain file name, when `upload_to` contains anything other than
	/// plain or `.` components, or when either contains `\`, `:`, or NUL.
	/// Returns [`FileFieldError::IoError`] when the write fails, including
	/// when a symlink would lead outside the storage root.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::new("uploads/files");
	/// let content = b"Hello, World!";
	/// let path = field.save("example.txt", content).unwrap();
	/// assert!(path.contains("uploads/files/example.txt"));
	/// ```
	pub fn save(&self, file_name: &str, content: &[u8]) -> Result<String, FileFieldError> {
		validate_upload_path(&self.upload_to, file_name)?;
		save_confined(
			self.storage_path.as_deref(),
			&self.upload_to,
			file_name,
			content,
		)
	}

	/// Generate URL for the file
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::new("uploads/files");
	/// let url = field.url("uploads/files/example.txt");
	/// assert_eq!(url, "/media/uploads/files/example.txt");
	/// ```
	pub fn url(&self, path: &str) -> String {
		format!("/media/{}", path.trim_start_matches('/'))
	}

	/// Check if file exists
	///
	/// `path` is interpreted relative to the storage root. Paths that are
	/// empty, absolute, prefixed, contain `..`, or resolve outside the storage
	/// root through a symlink are reported as missing.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::new("uploads/files");
	/// let exists = field.exists("uploads/files/example.txt");
	/// ```
	pub fn exists(&self, path: &str) -> bool {
		exists_confined(self.storage_path.as_deref(), path)
	}

	/// Delete file from storage
	///
	/// `path` is interpreted relative to the storage root. A missing file is
	/// treated as already deleted.
	///
	/// # Errors
	///
	/// Returns [`FileFieldError::InvalidPath`] when `path` is empty,
	/// absolute, has a platform prefix, or contains a `..` component. Returns
	/// [`FileFieldError::IoError`] when removal fails, including when a
	/// symlink would lead outside the storage root.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::FileField;
	///
	/// let field = FileField::new("uploads/files");
	/// field.delete("uploads/files/example.txt").unwrap();
	/// ```
	pub fn delete(&self, path: &str) -> Result<(), FileFieldError> {
		delete_confined(self.storage_path.as_deref(), path)
	}
}

impl Field for FileField {
	fn deconstruct(&self) -> FieldDeconstruction {
		let mut kwargs = self.base.get_kwargs();
		kwargs.insert(
			"upload_to".into(),
			FieldKwarg::String(self.upload_to.clone()),
		);

		if self.max_length != 100 {
			kwargs.insert("max_length".into(), FieldKwarg::Uint(self.max_length));
		}

		FieldDeconstruction {
			name: self.base.name.clone(),
			path: "reinhardt.orm.models.FileField".into(),
			args: vec![],
			kwargs,
		}
	}

	fn set_attributes_from_name(&mut self, name: &str) {
		self.base.name = Some(name.into());
	}

	fn name(&self) -> Option<&str> {
		self.base.name.as_deref()
	}
}

/// ImageField - file field specialized for images with validation
///
/// # Examples
///
/// ```
/// use reinhardt_db::orm::file_fields::ImageField;
///
/// let field = ImageField::new("uploads/images");
/// assert_eq!(field.upload_to, "uploads/images");
/// assert_eq!(field.max_length, 100);
/// ```
#[derive(Debug, Clone)]
pub struct ImageField {
	/// The base.
	pub base: BaseField,
	/// The upload to.
	pub upload_to: String,
	/// The max length.
	pub max_length: u64,
	/// The storage path.
	pub storage_path: Option<PathBuf>,
	/// The width field.
	pub width_field: Option<String>,
	/// The height field.
	pub height_field: Option<String>,
}

impl ImageField {
	/// Create a new ImageField
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// assert_eq!(field.upload_to, "uploads/images");
	/// ```
	pub fn new(upload_to: impl Into<String>) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length: 100,
			storage_path: None,
			width_field: None,
			height_field: None,
		}
	}

	/// Create ImageField with dimension fields
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::with_dimensions("uploads/images", "width", "height");
	/// assert_eq!(field.width_field, Some("width".to_string()));
	/// assert_eq!(field.height_field, Some("height".to_string()));
	/// ```
	pub fn with_dimensions(
		upload_to: impl Into<String>,
		width_field: impl Into<String>,
		height_field: impl Into<String>,
	) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length: 100,
			storage_path: None,
			width_field: Some(width_field.into()),
			height_field: Some(height_field.into()),
		}
	}

	/// Create ImageField with custom storage
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::ImageField;
	/// use std::path::PathBuf;
	///
	/// let field = ImageField::with_storage("uploads/images", PathBuf::from("/var/www/media"));
	/// assert_eq!(field.storage_path, Some(PathBuf::from("/var/www/media")));
	/// ```
	pub fn with_storage(upload_to: impl Into<String>, storage_path: PathBuf) -> Self {
		Self {
			base: BaseField::new(),
			upload_to: upload_to.into(),
			max_length: 100,
			storage_path: Some(storage_path),
			width_field: None,
			height_field: None,
		}
	}

	/// Validate image format and get dimensions
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// // Load image content from a file
	/// let content = std::fs::read("path/to/image.png").unwrap();
	/// let (width, height) = field.validate_image(&content).unwrap();
	/// assert!(width > 0 && height > 0);
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn validate_image(&self, content: &[u8]) -> Result<(u32, u32), FileFieldError> {
		let img = image::load_from_memory(content)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to load image: {}", e)))?;

		Ok((img.width(), img.height()))
	}

	/// Save image with validation
	///
	/// The image is written to `upload_to/file_name` below the storage root
	/// (`storage_path`, or the current directory when unset).
	///
	/// # Errors
	///
	/// Returns [`FileFieldError::InvalidPath`] when `file_name` is not a
	/// single plain file name, when `upload_to` contains anything other than
	/// plain or `.` components, or when either contains `\`, `:`, or NUL; the
	/// path is checked before the image. Returns
	/// [`FileFieldError::InvalidImage`] when `content` is not a decodable image.
	/// Returns [`FileFieldError::IoError`] when the write fails, including when
	/// a symlink would lead outside the storage root.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// // Load image content from a file
	/// let content = std::fs::read("path/to/image.png").unwrap();
	/// let (path, width, height) = field.save("photo.png", &content).unwrap();
	/// assert!(path.contains("uploads/images/photo.png"));
	/// assert!(width > 0 && height > 0);
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn save(
		&self,
		file_name: &str,
		content: &[u8],
	) -> Result<(String, u32, u32), FileFieldError> {
		validate_upload_path(&self.upload_to, file_name)?;
		// Validate image first
		let (width, height) = self.validate_image(content)?;

		let relative_path = save_confined(
			self.storage_path.as_deref(),
			&self.upload_to,
			file_name,
			content,
		)?;
		Ok((relative_path, width, height))
	}

	/// Validate image dimensions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let result = field.validate_dimensions(800, 600, Some(800), Some(600));
	/// assert!(result.is_ok());
	///
	/// let result = field.validate_dimensions(1024, 768, Some(800), Some(600));
	/// assert!(result.is_err());
	/// ```
	pub fn validate_dimensions(
		&self,
		width: u32,
		height: u32,
		max_width: Option<u32>,
		max_height: Option<u32>,
	) -> Result<(), FileFieldError> {
		if let Some(max_w) = max_width
			&& width > max_w
		{
			return Err(FileFieldError::InvalidDimensions {
				expected: (max_w, max_height.unwrap_or(u32::MAX)),
				actual: (width, height),
			});
		}

		if let Some(max_h) = max_height
			&& height > max_h
		{
			return Err(FileFieldError::InvalidDimensions {
				expected: (max_width.unwrap_or(u32::MAX), max_h),
				actual: (width, height),
			});
		}

		Ok(())
	}

	/// Generate URL for the image
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let url = field.url("uploads/images/photo.png");
	/// assert_eq!(url, "/media/uploads/images/photo.png");
	/// ```
	pub fn url(&self, path: &str) -> String {
		format!("/media/{}", path.trim_start_matches('/'))
	}

	/// Check if image exists
	///
	/// `path` is interpreted relative to the storage root. Paths that are
	/// empty, absolute, prefixed, contain `..`, or resolve outside the storage
	/// root through a symlink are reported as missing.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let exists = field.exists("uploads/images/photo.png");
	/// ```
	pub fn exists(&self, path: &str) -> bool {
		exists_confined(self.storage_path.as_deref(), path)
	}

	/// Delete image from storage
	///
	/// `path` is interpreted relative to the storage root. A missing file is
	/// treated as already deleted.
	///
	/// # Errors
	///
	/// Returns [`FileFieldError::InvalidPath`] when `path` is empty,
	/// absolute, has a platform prefix, or contains a `..` component. Returns
	/// [`FileFieldError::IoError`] when removal fails, including when a
	/// symlink would lead outside the storage root.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// field.delete("uploads/images/photo.png").unwrap();
	/// ```
	pub fn delete(&self, path: &str) -> Result<(), FileFieldError> {
		delete_confined(self.storage_path.as_deref(), path)
	}

	/// Resize image to specified dimensions
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let content = std::fs::read("photo.jpg").unwrap();
	/// let resized = field.resize(&content, 800, 600).unwrap();
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn resize(
		&self,
		content: &[u8],
		width: u32,
		height: u32,
	) -> Result<Vec<u8>, FileFieldError> {
		let img = image::load_from_memory(content)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to load image: {}", e)))?;

		let resized = img.resize(width, height, image::imageops::FilterType::Lanczos3);

		let mut buffer = Vec::new();
		resized
			.write_to(
				&mut std::io::Cursor::new(&mut buffer),
				image::ImageFormat::Png,
			)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to encode image: {}", e)))?;

		Ok(buffer)
	}

	/// Crop image to specified dimensions
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let content = std::fs::read("photo.jpg").unwrap();
	/// let cropped = field.crop(&content, 100, 100, 400, 300).unwrap();
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn crop(
		&self,
		content: &[u8],
		x: u32,
		y: u32,
		width: u32,
		height: u32,
	) -> Result<Vec<u8>, FileFieldError> {
		let img = image::load_from_memory(content)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to load image: {}", e)))?;

		let cropped = img.crop_imm(x, y, width, height);

		let mut buffer = Vec::new();
		cropped
			.write_to(
				&mut std::io::Cursor::new(&mut buffer),
				image::ImageFormat::Png,
			)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to encode image: {}", e)))?;

		Ok(buffer)
	}

	/// Convert image to specified format
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	/// use image::ImageFormat;
	///
	/// let field = ImageField::new("uploads/images");
	/// let content = std::fs::read("photo.png").unwrap();
	/// let jpeg = field.convert_format(&content, ImageFormat::Jpeg).unwrap();
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn convert_format(
		&self,
		content: &[u8],
		format: image::ImageFormat,
	) -> Result<Vec<u8>, FileFieldError> {
		let img = image::load_from_memory(content)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to load image: {}", e)))?;

		let mut buffer = Vec::new();
		img.write_to(&mut std::io::Cursor::new(&mut buffer), format)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to encode image: {}", e)))?;

		Ok(buffer)
	}

	/// Create thumbnail of specified maximum dimensions (maintains aspect ratio)
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::file_fields::ImageField;
	///
	/// let field = ImageField::new("uploads/images");
	/// let content = std::fs::read("photo.jpg").unwrap();
	/// let thumbnail = field.thumbnail(&content, 150, 150).unwrap();
	/// ```
	#[cfg(feature = "image-fields")]
	pub fn thumbnail(
		&self,
		content: &[u8],
		max_width: u32,
		max_height: u32,
	) -> Result<Vec<u8>, FileFieldError> {
		let img = image::load_from_memory(content)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to load image: {}", e)))?;

		let thumbnail = img.thumbnail(max_width, max_height);

		let mut buffer = Vec::new();
		thumbnail
			.write_to(
				&mut std::io::Cursor::new(&mut buffer),
				image::ImageFormat::Png,
			)
			.map_err(|e| FileFieldError::InvalidImage(format!("Failed to encode image: {}", e)))?;

		Ok(buffer)
	}
}

impl Field for ImageField {
	fn deconstruct(&self) -> FieldDeconstruction {
		let mut kwargs = self.base.get_kwargs();
		kwargs.insert(
			"upload_to".into(),
			FieldKwarg::String(self.upload_to.clone()),
		);

		if self.max_length != 100 {
			kwargs.insert("max_length".into(), FieldKwarg::Uint(self.max_length));
		}

		if let Some(ref width_field) = self.width_field {
			kwargs.insert(
				"width_field".into(),
				FieldKwarg::String(width_field.clone()),
			);
		}

		if let Some(ref height_field) = self.height_field {
			kwargs.insert(
				"height_field".into(),
				FieldKwarg::String(height_field.clone()),
			);
		}

		FieldDeconstruction {
			name: self.base.name.clone(),
			path: "reinhardt.orm.models.ImageField".into(),
			args: vec![],
			kwargs,
		}
	}

	fn set_attributes_from_name(&mut self, name: &str) {
		self.base.name = Some(name.into());
	}

	fn name(&self) -> Option<&str> {
		self.base.name.as_deref()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::{fixture, rstest};

	// FileField tests
	#[test]
	fn test_file_field_new() {
		let field = FileField::new("uploads/documents");
		assert_eq!(field.upload_to, "uploads/documents");
		assert_eq!(field.max_length, 100);
		assert!(field.storage_path.is_none());
	}

	#[test]
	fn test_file_field_with_max_length() {
		let field = FileField::with_max_length("uploads/files", 255);
		assert_eq!(field.upload_to, "uploads/files");
		assert_eq!(field.max_length, 255);
	}

	#[test]
	fn test_file_field_with_storage() {
		let storage = PathBuf::from("/var/www/media");
		let field = FileField::with_storage("uploads/files", storage.clone());
		assert_eq!(field.storage_path, Some(storage));
	}

	#[test]
	fn test_file_field_url() {
		let field = FileField::new("uploads/files");
		let url = field.url("uploads/files/document.pdf");
		assert_eq!(url, "/media/uploads/files/document.pdf");

		// Test with leading slash
		let url2 = field.url("/uploads/files/document.pdf");
		assert_eq!(url2, "/media/uploads/files/document.pdf");
	}

	#[test]
	fn test_file_field_deconstruct() {
		let mut field = FileField::new("uploads/files");
		field.set_attributes_from_name("document");

		let dec = field.deconstruct();
		assert_eq!(dec.name, Some("document".into()));
		assert_eq!(dec.path, "reinhardt.orm.models.FileField");
		assert_eq!(
			dec.kwargs.get("upload_to"),
			Some(&FieldKwarg::String("uploads/files".into()))
		);
	}

	#[test]
	fn test_file_field_deconstruct_custom_max_length() {
		let field = FileField::with_max_length("uploads/files", 200);
		let dec = field.deconstruct();
		assert_eq!(dec.kwargs.get("max_length"), Some(&FieldKwarg::Uint(200)));
	}

	#[test]
	fn test_file_field_save_and_exists() {
		use std::env;

		let temp_dir = env::temp_dir().join("reinhardt_test_file_field");
		let field = FileField::with_storage("uploads/files", temp_dir.clone());

		let content = b"Test file content";
		let path = field.save("test.txt", content).unwrap();

		assert!(path.contains("uploads/files/test.txt"));
		assert!(field.exists(&path));

		// Cleanup
		field.delete(&path).ok();
		fs::remove_dir_all(temp_dir).ok();
	}

	#[rstest::rstest]
	fn file_field_rejects_traversal_before_creating_storage() {
		let storage = tempfile::tempdir().unwrap();
		let field = FileField::with_storage("uploads/files", storage.path().to_path_buf());

		for name in [
			"../escape.txt",
			"/tmp/escape.txt",
			"sub/file.txt",
			"..\\escape.txt",
			"C:escape.txt",
		] {
			assert!(matches!(
				field.save(name, b"payload"),
				Err(FileFieldError::InvalidPath(_))
			));
		}
		assert!(!storage.path().join("uploads").exists());
		assert!(!storage.path().join("escape.txt").exists());
	}

	#[rstest::rstest]
	#[case("./uploads")]
	#[case("./uploads/./files")]
	#[case(".")]
	fn file_field_accepts_current_directory_components(#[case] directory: &str) {
		let storage = tempfile::tempdir().unwrap();
		let field = FileField::with_storage(directory, storage.path().to_path_buf());

		let saved = field.save("safe.txt", b"payload").unwrap();

		assert_eq!(fs::read(storage.path().join(&saved)).unwrap(), b"payload");
		assert!(field.exists(&saved));
		assert!(!Path::new(&saved).is_absolute());
		assert!(matches!(
			field.save("../unsafe.txt", b"payload"),
			Err(FileFieldError::InvalidPath(_))
		));
	}

	#[rstest::rstest]
	fn file_field_rejects_unsafe_upload_directory() {
		let storage = tempfile::tempdir().unwrap();
		let field = FileField::with_storage("../outside", storage.path().to_path_buf());
		assert!(matches!(
			field.save("safe.txt", b"payload"),
			Err(FileFieldError::InvalidPath(_))
		));
	}

	#[test]
	fn test_file_field_delete() {
		use std::env;

		let temp_dir = env::temp_dir().join("reinhardt_test_file_field_delete");
		let field = FileField::with_storage("uploads/files", temp_dir.clone());

		let content = b"Test file content";
		let path = field.save("test_delete.txt", content).unwrap();

		assert!(field.exists(&path));

		field.delete(&path).unwrap();
		assert!(!field.exists(&path));

		// Cleanup
		fs::remove_dir_all(temp_dir).ok();
	}

	// ImageField tests
	#[test]
	fn test_image_field_new() {
		let field = ImageField::new("uploads/images");
		assert_eq!(field.upload_to, "uploads/images");
		assert_eq!(field.max_length, 100);
		assert!(field.width_field.is_none());
		assert!(field.height_field.is_none());
	}

	#[test]
	fn test_image_field_with_dimensions() {
		let field = ImageField::with_dimensions("uploads/images", "img_width", "img_height");
		assert_eq!(field.width_field, Some("img_width".into()));
		assert_eq!(field.height_field, Some("img_height".into()));
	}

	#[test]
	fn test_image_field_with_storage() {
		let storage = PathBuf::from("/var/www/media");
		let field = ImageField::with_storage("uploads/images", storage.clone());
		assert_eq!(field.storage_path, Some(storage));
	}

	#[test]
	fn test_image_field_url() {
		let field = ImageField::new("uploads/images");
		let url = field.url("uploads/images/photo.png");
		assert_eq!(url, "/media/uploads/images/photo.png");
	}

	#[test]
	fn test_image_field_validate_dimensions_valid() {
		let field = ImageField::new("uploads/images");
		let result = field.validate_dimensions(800, 600, Some(1024), Some(768));
		assert!(result.is_ok());
	}

	#[test]
	fn test_image_field_validate_dimensions_invalid_width() {
		let field = ImageField::new("uploads/images");
		let result = field.validate_dimensions(1200, 600, Some(1024), Some(768));
		assert!(result.is_err());

		if let Err(FileFieldError::InvalidDimensions { expected, actual }) = result {
			assert_eq!(expected.0, 1024);
			assert_eq!(actual.0, 1200);
		} else {
			panic!("Expected InvalidDimensions error");
		}
	}

	#[test]
	fn test_image_field_validate_dimensions_invalid_height() {
		let field = ImageField::new("uploads/images");
		let result = field.validate_dimensions(800, 900, Some(1024), Some(768));
		assert!(result.is_err());

		if let Err(FileFieldError::InvalidDimensions { expected, actual }) = result {
			assert_eq!(expected.1, 768);
			assert_eq!(actual.1, 900);
		} else {
			panic!("Expected InvalidDimensions error");
		}
	}

	#[test]
	fn test_image_field_validate_dimensions_no_limit() {
		let field = ImageField::new("uploads/images");
		let result = field.validate_dimensions(2000, 2000, None, None);
		assert!(result.is_ok());
	}

	#[test]
	fn test_image_field_deconstruct() {
		let mut field = ImageField::new("uploads/images");
		field.set_attributes_from_name("photo");

		let dec = field.deconstruct();
		assert_eq!(dec.name, Some("photo".into()));
		assert_eq!(dec.path, "reinhardt.orm.models.ImageField");
		assert_eq!(
			dec.kwargs.get("upload_to"),
			Some(&FieldKwarg::String("uploads/images".into()))
		);
	}

	#[test]
	fn test_image_field_deconstruct_with_dimensions() {
		let field = ImageField::with_dimensions("uploads/images", "width", "height");
		let dec = field.deconstruct();

		assert_eq!(
			dec.kwargs.get("width_field"),
			Some(&FieldKwarg::String("width".into()))
		);
		assert_eq!(
			dec.kwargs.get("height_field"),
			Some(&FieldKwarg::String("height".into()))
		);
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_validate_image_invalid() {
		let field = ImageField::new("uploads/images");
		let invalid_content = b"Not an image";
		let result = field.validate_image(invalid_content);
		assert!(result.is_err());
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_resize() {
		use image::{ImageBuffer, Rgb};

		let field = ImageField::new("uploads/images");

		// Create a simple 100x100 red image
		let img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
		let mut original = Vec::new();
		img.write_to(
			&mut std::io::Cursor::new(&mut original),
			image::ImageFormat::Png,
		)
		.unwrap();

		// Resize to 50x50
		let resized_content = field.resize(&original, 50, 50).unwrap();

		// Verify the resized image dimensions
		let resized_img = image::load_from_memory(&resized_content).unwrap();
		assert_eq!(resized_img.width(), 50);
		assert_eq!(resized_img.height(), 50);
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_crop() {
		use image::{ImageBuffer, Rgb};

		let field = ImageField::new("uploads/images");

		// Create a simple 100x100 image
		let img = ImageBuffer::from_fn(100, 100, |x, y| {
			if x < 50 && y < 50 {
				Rgb([255u8, 0u8, 0u8]) // Red top-left
			} else {
				Rgb([0u8, 0u8, 255u8]) // Blue elsewhere
			}
		});
		let mut original = Vec::new();
		img.write_to(
			&mut std::io::Cursor::new(&mut original),
			image::ImageFormat::Png,
		)
		.unwrap();

		// Crop 50x50 from top-left
		let cropped_content = field.crop(&original, 0, 0, 50, 50).unwrap();

		// Verify dimensions
		let cropped_img = image::load_from_memory(&cropped_content).unwrap();
		assert_eq!(cropped_img.width(), 50);
		assert_eq!(cropped_img.height(), 50);
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_convert_format() {
		use image::{ImageBuffer, ImageFormat, Rgb};

		let field = ImageField::new("uploads/images");

		// Create a simple image in PNG format
		let img = ImageBuffer::from_fn(50, 50, |_, _| Rgb([255u8, 0u8, 0u8]));
		let mut original = Vec::new();
		img.write_to(&mut std::io::Cursor::new(&mut original), ImageFormat::Png)
			.unwrap();

		// Convert to JPEG
		let jpeg_content = field.convert_format(&original, ImageFormat::Jpeg).unwrap();

		// Verify it can be loaded
		let jpeg_img = image::load_from_memory(&jpeg_content).unwrap();
		assert_eq!(jpeg_img.width(), 50);
		assert_eq!(jpeg_img.height(), 50);
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_thumbnail() {
		use image::{ImageBuffer, Rgb};

		let field = ImageField::new("uploads/images");

		// Create a 200x100 image
		let img = ImageBuffer::from_fn(200, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
		let mut original = Vec::new();
		img.write_to(
			&mut std::io::Cursor::new(&mut original),
			image::ImageFormat::Png,
		)
		.unwrap();

		// Create thumbnail with max 80x80 (should maintain aspect ratio: 80x40)
		let thumbnail_content = field.thumbnail(&original, 80, 80).unwrap();

		// Verify dimensions (aspect ratio maintained)
		let thumbnail_img = image::load_from_memory(&thumbnail_content).unwrap();
		assert_eq!(thumbnail_img.width(), 80);
		assert_eq!(thumbnail_img.height(), 40);
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_resize_invalid_content() {
		let field = ImageField::new("uploads/images");
		let invalid_content = b"Not an image";
		let result = field.resize(invalid_content, 100, 100);
		assert!(result.is_err());
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_crop_invalid_content() {
		let field = ImageField::new("uploads/images");
		let invalid_content = b"Not an image";
		let result = field.crop(invalid_content, 0, 0, 50, 50);
		assert!(result.is_err());
	}

	#[cfg(feature = "image-fields")]
	#[test]
	fn test_image_field_convert_format_invalid_content() {
		let field = ImageField::new("uploads/images");
		let invalid_content = b"Not an image";
		let result = field.convert_format(invalid_content, image::ImageFormat::Jpeg);
		assert!(result.is_err());
	}

	/// Storage root nested inside a parent directory, so files that land next
	/// to the root are observable escapes.
	struct ConfinedLayout {
		parent: tempfile::TempDir,
		root: PathBuf,
	}

	#[fixture]
	fn layout() -> ConfinedLayout {
		let parent = tempfile::TempDir::new().expect("temporary directory should be created");
		let root = parent.path().join("media");
		fs::create_dir(&root).expect("storage root should be created");
		ConfinedLayout { parent, root }
	}

	#[rstest]
	#[case::parent_escape("../../escape.txt")]
	#[case::nested_parent_escape("nested/../../../escape.txt")]
	#[case::empty("")]
	#[case::current_dir(".")]
	fn file_field_save_rejects_names_outside_upload_root(
		layout: ConfinedLayout,
		#[case] file_name: &str,
	) {
		// Arrange
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let result = field.save(file_name, b"payload");

		// Assert
		assert!(matches!(result, Err(FileFieldError::InvalidPath(ref path)) if path == file_name));
		assert!(!layout.parent.path().join("escape.txt").exists());
	}

	#[rstest]
	fn file_field_save_rejects_absolute_file_name(layout: ConfinedLayout) {
		// Arrange
		let target = layout.parent.path().join("escape.txt");
		let file_name = target.to_str().expect("temporary path should be UTF-8");
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let result = field.save(file_name, b"payload");

		// Assert
		assert!(matches!(result, Err(FileFieldError::InvalidPath(ref path)) if path == file_name));
		assert!(!target.exists());
	}

	#[rstest]
	fn file_field_save_rejects_upload_to_outside_storage_root(layout: ConfinedLayout) {
		// Arrange
		let field = FileField::with_storage("../outside", layout.root.clone());

		// Act
		let result = field.save("file.txt", b"payload");

		// Assert
		assert!(matches!(
			result,
			Err(FileFieldError::InvalidPath(ref path)) if path == "../outside"
		));
		assert!(!layout.parent.path().join("outside").exists());
	}

	#[rstest]
	#[case::parent_escape("../victim.txt")]
	#[case::nested_parent_escape("uploads/../../victim.txt")]
	fn file_field_delete_and_exists_reject_paths_outside_storage_root(
		layout: ConfinedLayout,
		#[case] path: &str,
	) {
		// Arrange
		let victim = layout.parent.path().join("victim.txt");
		fs::write(&victim, b"keep").expect("victim fixture should be written");
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let exists = field.exists(path);
		let deleted = field.delete(path);

		// Assert
		assert!(!exists);
		assert!(
			matches!(deleted, Err(FileFieldError::InvalidPath(ref rejected)) if rejected == path)
		);
		assert_eq!(fs::read(&victim).expect("victim should remain"), b"keep");
	}

	#[rstest]
	fn file_field_delete_and_exists_reject_absolute_path(layout: ConfinedLayout) {
		// Arrange
		let victim = layout.parent.path().join("victim.txt");
		fs::write(&victim, b"keep").expect("victim fixture should be written");
		let path = victim.to_str().expect("temporary path should be UTF-8");
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let exists = field.exists(path);
		let deleted = field.delete(path);

		// Assert
		assert!(!exists);
		assert!(
			matches!(deleted, Err(FileFieldError::InvalidPath(ref rejected)) if rejected == path)
		);
		assert_eq!(fs::read(&victim).expect("victim should remain"), b"keep");
	}

	#[cfg(unix)]
	#[rstest]
	fn file_field_rejects_directory_symlinks_that_escape_storage_root(layout: ConfinedLayout) {
		use std::os::unix::fs::symlink;

		// Arrange
		let outside = layout.parent.path().join("outside");
		fs::create_dir(&outside).expect("outside directory should be created");
		fs::write(outside.join("victim.txt"), b"keep").expect("victim fixture should be written");
		symlink(&outside, layout.root.join("uploads")).expect("test symlink should be created");
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let saved = field.save("new.txt", b"payload");
		let exists = field.exists("uploads/victim.txt");
		let deleted = field.delete("uploads/victim.txt");

		// Assert
		assert!(matches!(saved, Err(FileFieldError::IoError(_))));
		assert!(!exists);
		assert!(matches!(deleted, Err(FileFieldError::IoError(_))));
		assert!(!outside.join("new.txt").exists());
		assert_eq!(
			fs::read(outside.join("victim.txt")).expect("victim should remain"),
			b"keep"
		);
	}

	#[cfg(unix)]
	#[rstest]
	fn file_field_save_rejects_dangling_symlink_that_escapes_storage_root(layout: ConfinedLayout) {
		use std::os::unix::fs::symlink;

		// Arrange
		let target = layout.parent.path().join("escape.txt");
		fs::create_dir(layout.root.join("uploads")).expect("upload directory should be created");
		symlink(&target, layout.root.join("uploads").join("link.txt"))
			.expect("test symlink should be created");
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let result = field.save("link.txt", b"payload");

		// Assert
		assert!(matches!(result, Err(FileFieldError::IoError(_))));
		assert!(!target.exists());
	}

	#[rstest]
	#[case::flat("uploads", "report.txt", "uploads/report.txt")]
	#[case::nested("uploads/2026/10", "report.txt", "uploads/2026/10/report.txt")]
	#[case::double_dot_in_name("uploads", "archive..tar", "uploads/archive..tar")]
	fn file_field_round_trips_paths_inside_storage_root(
		layout: ConfinedLayout,
		#[case] upload_to: &str,
		#[case] file_name: &str,
		#[case] expected: &str,
	) {
		// Arrange
		let field = FileField::with_storage(upload_to, layout.root.clone());

		// Act
		let saved = field
			.save(file_name, b"payload")
			.expect("confined save should succeed");
		let stored = fs::read(layout.root.join(&saved)).expect("saved file should be readable");
		let existed = field.exists(&saved);
		field
			.delete(&saved)
			.expect("confined delete should succeed");

		// Assert
		assert_eq!(saved, expected);
		assert_eq!(stored, b"payload");
		assert!(existed);
		assert!(!field.exists(&saved));
	}

	#[rstest]
	fn file_field_delete_treats_missing_file_as_deleted(layout: ConfinedLayout) {
		// Arrange
		let field = FileField::with_storage("uploads", layout.root.clone());

		// Act
		let result = field.delete("uploads/missing.txt");

		// Assert
		assert!(matches!(result, Ok(())));
	}

	#[rstest]
	fn image_field_delete_and_exists_reject_paths_outside_storage_root(layout: ConfinedLayout) {
		// Arrange
		let victim = layout.parent.path().join("victim.png");
		fs::write(&victim, b"keep").expect("victim fixture should be written");
		let field = ImageField::with_storage("uploads", layout.root.clone());

		// Act
		let exists = field.exists("../victim.png");
		let deleted = field.delete("../victim.png");

		// Assert
		assert!(!exists);
		assert!(matches!(
			deleted,
			Err(FileFieldError::InvalidPath(ref rejected)) if rejected == "../victim.png"
		));
		assert_eq!(fs::read(&victim).expect("victim should remain"), b"keep");
	}

	#[cfg(feature = "image-fields")]
	#[rstest]
	fn image_field_save_confines_names_to_upload_root(layout: ConfinedLayout) {
		use image::{ImageBuffer, Rgb};

		// Arrange
		let img = ImageBuffer::from_fn(4, 4, |_, _| Rgb([255u8, 0u8, 0u8]));
		let mut png = Vec::new();
		img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
			.expect("test image should be encoded");
		let field = ImageField::with_storage("uploads", layout.root.clone());

		// Act
		let rejected = field.save("../../escape.png", &png);
		let saved = field
			.save("photo.png", &png)
			.expect("confined image save should succeed");

		// Assert
		assert!(matches!(
			rejected,
			Err(FileFieldError::InvalidPath(ref path)) if path == "../../escape.png"
		));
		assert!(!layout.parent.path().join("escape.png").exists());
		assert_eq!(saved, ("uploads/photo.png".to_string(), 4, 4));
		assert_eq!(
			fs::read(layout.root.join("uploads/photo.png")).expect("saved image should exist"),
			png
		);
	}
}
