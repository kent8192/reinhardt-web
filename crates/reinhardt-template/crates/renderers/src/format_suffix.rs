//! Format suffix extraction utilities
//!
//! This module provides utilities for extracting format suffixes from URL paths,
//! such as `.json`, `.xml`, `.yaml`, etc.
//!
//! # Examples
//!
//! ```
//! use reinhardt_renderers::format_suffix::extract_format_suffix;
//!
//! let (path, format) = extract_format_suffix("/api/users.json");
//! assert_eq!(path, "/api/users");
//! assert_eq!(format, Some("json"));
//!
//! let (path, format) = extract_format_suffix("/api/users");
//! assert_eq!(path, "/api/users");
//! assert_eq!(format, None);
//! ```

use std::collections::HashMap;

/// Mapping of format suffixes to media types
fn get_format_to_media_type_map() -> HashMap<&'static str, &'static str> {
	let mut map = HashMap::new();
	map.insert("json", "application/json");
	map.insert("xml", "application/xml");
	map.insert("yaml", "application/yaml");
	map.insert("yml", "application/yaml");
	map.insert("csv", "text/csv");
	map.insert("html", "text/html");
	map.insert("txt", "text/plain");
	map
}

/// Extracts format suffix from a URL path
///
/// Returns a tuple of (path_without_suffix, format)
/// where format is None if no recognized suffix is found.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::format_suffix::extract_format_suffix;
///
/// let (path, format) = extract_format_suffix("/api/users.json");
/// assert_eq!(path, "/api/users");
/// assert_eq!(format, Some("json"));
///
/// let (path, format) = extract_format_suffix("/api/users.xml");
/// assert_eq!(path, "/api/users");
/// assert_eq!(format, Some("xml"));
///
/// let (path, format) = extract_format_suffix("/api/users");
/// assert_eq!(path, "/api/users");
/// assert_eq!(format, None);
///
/// let (path, format) = extract_format_suffix("/api/users.unknown");
/// assert_eq!(path, "/api/users.unknown");
/// assert_eq!(format, None);
/// ```
pub fn extract_format_suffix(path: &str) -> (&str, Option<&str>) {
	// Find the last dot in the path
	if let Some(dot_index) = path.rfind('.') {
		// Extract the suffix (after the dot)
		let suffix = &path[dot_index + 1..];

		// Check if this is a recognized format
		let format_map = get_format_to_media_type_map();
		if format_map.contains_key(suffix) {
			// Return path without suffix and the format
			return (&path[..dot_index], Some(suffix));
		}
	}

	// No recognized suffix found
	(path, None)
}

/// Gets the media type for a given format suffix
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::format_suffix::get_media_type_for_format;
///
/// assert_eq!(get_media_type_for_format("json"), Some("application/json"));
/// assert_eq!(get_media_type_for_format("xml"), Some("application/xml"));
/// assert_eq!(get_media_type_for_format("yaml"), Some("application/yaml"));
/// assert_eq!(get_media_type_for_format("yml"), Some("application/yaml"));
/// assert_eq!(get_media_type_for_format("unknown"), None);
/// ```
pub fn get_media_type_for_format(format: &str) -> Option<&'static str> {
	let format_map = get_format_to_media_type_map();
	format_map.get(format).copied()
}

/// Checks if a format suffix is supported
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::format_suffix::is_supported_format;
///
/// assert!(is_supported_format("json"));
/// assert!(is_supported_format("xml"));
/// assert!(is_supported_format("yaml"));
/// assert!(is_supported_format("yml"));
/// assert!(!is_supported_format("unknown"));
/// ```
pub fn is_supported_format(format: &str) -> bool {
	let format_map = get_format_to_media_type_map();
	format_map.contains_key(format)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_format_suffix_json() {
		let (path, format) = extract_format_suffix("/api/users.json");
		assert_eq!(path, "/api/users");
		assert_eq!(format, Some("json"));
	}

	#[test]
	fn test_extract_format_suffix_xml() {
		let (path, format) = extract_format_suffix("/api/users.xml");
		assert_eq!(path, "/api/users");
		assert_eq!(format, Some("xml"));
	}

	#[test]
	fn test_extract_format_suffix_yaml() {
		let (path, format) = extract_format_suffix("/api/users.yaml");
		assert_eq!(path, "/api/users");
		assert_eq!(format, Some("yaml"));
	}

	#[test]
	fn test_extract_format_suffix_yml() {
		let (path, format) = extract_format_suffix("/api/users.yml");
		assert_eq!(path, "/api/users");
		assert_eq!(format, Some("yml"));
	}

	#[test]
	fn test_extract_format_suffix_none() {
		let (path, format) = extract_format_suffix("/api/users");
		assert_eq!(path, "/api/users");
		assert_eq!(format, None);
	}

	#[test]
	fn test_extract_format_suffix_unknown() {
		let (path, format) = extract_format_suffix("/api/users.unknown");
		assert_eq!(path, "/api/users.unknown");
		assert_eq!(format, None);
	}

	#[test]
	fn test_extract_format_suffix_nested_path() {
		let (path, format) = extract_format_suffix("/api/v1/users/123.json");
		assert_eq!(path, "/api/v1/users/123");
		assert_eq!(format, Some("json"));
	}

	#[test]
	fn test_get_media_type_for_format_json() {
		assert_eq!(get_media_type_for_format("json"), Some("application/json"));
	}

	#[test]
	fn test_get_media_type_for_format_xml() {
		assert_eq!(get_media_type_for_format("xml"), Some("application/xml"));
	}

	#[test]
	fn test_get_media_type_for_format_yaml() {
		assert_eq!(get_media_type_for_format("yaml"), Some("application/yaml"));
	}

	#[test]
	fn test_get_media_type_for_format_yml() {
		assert_eq!(get_media_type_for_format("yml"), Some("application/yaml"));
	}

	#[test]
	fn test_get_media_type_for_format_unknown() {
		assert_eq!(get_media_type_for_format("unknown"), None);
	}

	#[test]
	fn test_is_supported_format_json() {
		assert!(is_supported_format("json"));
	}

	#[test]
	fn test_is_supported_format_xml() {
		assert!(is_supported_format("xml"));
	}

	#[test]
	fn test_is_supported_format_unknown() {
		assert!(!is_supported_format("unknown"));
	}
}
