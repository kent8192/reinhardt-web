//! Runtime support for asset! macro.
//!
//! This module provides runtime URL resolution for static files using a manifest.
//! The manifest is typically generated during the build process (e.g., by collectstatic)
//! and contains mappings from original file paths to hashed/versioned file paths.
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::integ::static_context;
//! use std::collections::HashMap;
//!
//! // Initialize with manifest (typically done in main.rs)
//! let mut manifest = HashMap::new();
//! manifest.insert("images/logo.png".to_string(), "images/logo.abc123.png".to_string());
//! static_context::init_static_context(manifest);
//!
//! // Resolve URLs (automatically called by asset! macro)
//! let url = static_context::resolve_static_url("images/logo.png");
//! assert_eq!(url, "/static/images/logo.abc123.png");
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global static file manifest.
///
/// This manifest maps original file paths to hashed/versioned file paths.
/// It should be initialized once at application startup using [`init_static_context`].
static STATIC_MANIFEST: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Initializes the static file context with a manifest.
///
/// This function should be called once at application startup, typically in main.rs.
/// Subsequent calls will panic.
///
/// # Panics
///
/// Panics if the static context has already been initialized.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages::integ::static_context;
/// use std::collections::HashMap;
///
/// let mut manifest = HashMap::new();
/// manifest.insert("css/style.css".to_string(), "css/style.abc123.css".to_string());
/// static_context::init_static_context(manifest);
/// ```
pub fn init_static_context(manifest: HashMap<String, String>) {
	STATIC_MANIFEST
		.set(manifest)
		.expect("Static context already initialized");
}

/// Resolves a static file path to its versioned URL.
///
/// This function looks up the given path in the static file manifest and returns
/// the corresponding versioned URL. If the path is not found in the manifest,
/// it returns the original path with `/static/` prefix.
///
/// # Panics
///
/// Panics if the static context has not been initialized with [`init_static_context`].
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages::integ::static_context;
///
/// // With manifest entry
/// let url = static_context::resolve_static_url("images/logo.png");
/// assert_eq!(url, "/static/images/logo.abc123.png");
///
/// // Without manifest entry (fallback)
/// let url = static_context::resolve_static_url("unknown.png");
/// assert_eq!(url, "/static/unknown.png");
/// ```
pub fn resolve_static_url(path: &str) -> String {
	let manifest = STATIC_MANIFEST
		.get()
		.expect("Static context not initialized. Call init_static_context() first.");

	// Look up in manifest, or fallback to original path
	manifest
		.get(path)
		.map(|hashed_path| format!("/static/{}", hashed_path))
		.unwrap_or_else(|| format!("/static/{}", path))
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serial_test::serial;

	#[rstest]
	#[serial(static_context)]
	fn test_resolve_with_manifest() {
		let mut manifest = HashMap::new();
		manifest.insert(
			"images/logo.png".to_string(),
			"images/logo.abc123.png".to_string(),
		);
		manifest.insert(
			"css/style.css".to_string(),
			"css/style.def456.css".to_string(),
		);

		init_static_context(manifest);

		assert_eq!(
			resolve_static_url("images/logo.png"),
			"/static/images/logo.abc123.png"
		);
		assert_eq!(
			resolve_static_url("css/style.css"),
			"/static/css/style.def456.css"
		);
	}

	#[rstest]
	#[serial(static_context)]
	fn test_resolve_fallback() {
		// Initialize context if not already initialized (OnceLock can only be set once)
		let _ = STATIC_MANIFEST.set(HashMap::new());

		assert_eq!(resolve_static_url("unknown.png"), "/static/unknown.png");
	}

	#[rstest]
	#[should_panic(expected = "Static context not initialized")]
	fn test_resolve_before_init() {
		// Note: This test will fail if run after other tests that initialize the context
		// In practice, use a separate test binary or reset mechanism
		resolve_static_url("test.png");
	}
}
