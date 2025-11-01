use mime_guess::from_path;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StaticError {
	#[error("File not found: {0}")]
	NotFound(String),
	#[error("Directory traversal blocked: {0}")]
	DirectoryTraversal(String),
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
}

#[derive(Debug)]
pub struct StaticFile {
	pub content: Vec<u8>,
	pub path: PathBuf,
	pub mime_type: String,
}

impl StaticFile {
	/// Generate ETag for the file based on content hash
	pub fn etag(&self) -> String {
		use std::collections::hash_map::DefaultHasher;
		use std::hash::{Hash, Hasher};

		let mut hasher = DefaultHasher::new();
		self.content.hash(&mut hasher);
		format!("\"{}\"", hasher.finish())
	}
}

pub struct StaticFileHandler {
	root: PathBuf,
	index_files: Vec<String>,
}

impl StaticFileHandler {
	pub fn new(root: PathBuf) -> Self {
		Self {
			root,
			index_files: vec!["index.html".to_string()],
		}
	}

	/// Configure custom index files to serve for directories
	///
	/// When a directory is requested, the handler will try to serve
	/// the first matching index file from this list.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_static::handler::StaticFileHandler;
	/// use std::path::PathBuf;
	///
	/// let handler = StaticFileHandler::new(PathBuf::from("static"))
	///     .with_index_files(vec!["index.html".to_string(), "default.html".to_string()]);
	/// ```
	pub fn with_index_files(mut self, index_files: Vec<String>) -> Self {
		self.index_files = index_files;
		self
	}

	pub async fn serve(&self, path: &str) -> Result<StaticFile, StaticError> {
		let resolved = self.resolve_path(path).await?;
		let content = fs::read(&resolved)?;
		let mime_type = from_path(&resolved).first_or_octet_stream().to_string();

		Ok(StaticFile {
			content,
			path: resolved,
			mime_type,
		})
	}

	pub async fn resolve_path(&self, path: &str) -> Result<PathBuf, StaticError> {
		let path = path.trim_start_matches('/');

		// Prevent directory traversal attacks
		if path.contains("..") {
			return Err(StaticError::DirectoryTraversal(path.to_string()));
		}

		let file_path = self.root.join(path);

		// Canonicalize paths to prevent traversal
		let canonical_file = file_path
			.canonicalize()
			.map_err(|_| StaticError::NotFound(path.to_string()))?;
		let canonical_root = self.root.canonicalize().map_err(StaticError::Io)?;

		// Ensure file is within root directory
		if !canonical_file.starts_with(&canonical_root) {
			return Err(StaticError::DirectoryTraversal(path.to_string()));
		}

		// If it's a directory, try to serve an index file
		if canonical_file.is_dir() {
			for index_file in &self.index_files {
				let index_path = canonical_file.join(index_file);
				if index_path.exists() && index_path.is_file() {
					return Ok(index_path);
				}
			}
			// No index file found in directory
			return Err(StaticError::NotFound(format!(
				"{} (directory without index file)",
				path
			)));
		}

		Ok(canonical_file)
	}
}

pub type StaticResult<T> = Result<T, StaticError>;
