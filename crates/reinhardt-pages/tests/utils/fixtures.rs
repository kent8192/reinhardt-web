//! Fixture loading utilities
//!
//! Provides functions to load test fixtures from JSON files.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_pages_test_utils::load_fixture;
//!
//! let json = load_fixture("forms/simple_form.json");
//! let form: MyForm = serde_json::from_str(&json).unwrap();
//! ```

use std::path::PathBuf;

/// Base path for fixtures
fn fixtures_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Loads a fixture file as a string
///
/// # Arguments
///
/// * `path` - Path relative to the fixtures directory (e.g., "forms/simple_form.json")
///
/// # Panics
///
/// Panics if the file cannot be read
///
/// # Example
///
/// ```rust
/// let json = load_fixture("forms/simple_form.json");
/// ```
pub fn load_fixture(path: &str) -> String {
	let full_path = fixtures_dir().join(path);
	std::fs::read_to_string(&full_path)
		.unwrap_or_else(|e| panic!("Failed to load fixture at {:?}: {}", full_path.display(), e))
}

/// Loads a JSON fixture and deserializes it
///
/// # Arguments
///
/// * `path` - Path relative to the fixtures directory
///
/// # Panics
///
/// Panics if the file cannot be read or deserialized
///
/// # Example
///
/// ```rust
/// #[derive(Deserialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user: User = load_json_fixture("auth/user.json");
/// ```
pub fn load_json_fixture<T: serde::de::DeserializeOwned>(path: &str) -> T {
	let content = load_fixture(path);
	serde_json::from_str(&content)
		.unwrap_or_else(|e| panic!("Failed to parse JSON fixture at {}: {}", path, e))
}

/// Lists all fixtures in a directory
///
/// # Arguments
///
/// * `dir` - Directory relative to the fixtures directory
///
/// # Returns
///
/// A vector of fixture file names (without directory prefix)
///
/// # Example
///
/// ```rust
/// let forms = list_fixtures("forms");
/// assert!(forms.contains(&"simple_form.json".to_string()));
/// ```
pub fn list_fixtures(dir: &str) -> Vec<String> {
	let full_path = fixtures_dir().join(dir);

	if !full_path.exists() {
		return Vec::new();
	}

	std::fs::read_dir(&full_path)
		.unwrap_or_else(|e| panic!("Failed to read fixture directory {:?}: {}", full_path, e))
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let file_name = entry.file_name().to_string_lossy().to_string();
			if entry.file_type().ok()?.is_file() {
				Some(file_name)
			} else {
				None
			}
		})
		.collect()
}

/// Checks if a fixture exists
///
/// # Arguments
///
/// * `path` - Path relative to the fixtures directory
///
/// # Example
///
/// ```rust
/// if fixture_exists("forms/complex_form.json") {
///     let json = load_fixture("forms/complex_form.json");
/// }
/// ```
pub fn fixture_exists(path: &str) -> bool {
	fixtures_dir().join(path).exists()
}

#[cfg(test)]
mod tests {
	use rstest::rstest;
	use super::*;

	#[rstest]
	fn test_fixtures_dir() {
		let dir = fixtures_dir();
		assert!(dir.ends_with("tests/fixtures"));
	}

	#[rstest]
	#[should_panic(expected = "Failed to load fixture")]
	fn test_load_fixture_missing() {
		load_fixture("nonexistent.json");
	}

	#[rstest]
	fn test_fixture_exists() {
		// This will be false until we create actual fixtures
		let exists = fixture_exists("test.json");
		assert!(!exists);
	}
}
