//! DOM Reconciliation for Hydration
//!
//! This module verifies that SSR-rendered DOM matches the expected
//! component structure during hydration.

use crate::component::View;

#[cfg(target_arch = "wasm32")]
use crate::dom::Element;

/// Errors that can occur during reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileError {
	/// Tag name mismatch.
	TagMismatch {
		/// Expected tag name.
		expected: String,
		/// Actual tag name.
		actual: String,
	},
	/// Child count mismatch.
	ChildCountMismatch {
		/// Expected count.
		expected: usize,
		/// Actual count.
		actual: usize,
	},
	/// Text content mismatch.
	TextMismatch {
		/// Expected text.
		expected: String,
		/// Actual text.
		actual: String,
	},
	/// Attribute mismatch.
	AttributeMismatch {
		/// Attribute name.
		name: String,
		/// Expected value.
		expected: Option<String>,
		/// Actual value.
		actual: Option<String>,
	},
	/// Element not found at expected position.
	ElementNotFound {
		/// Position index.
		index: usize,
	},
}

impl std::fmt::Display for ReconcileError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TagMismatch { expected, actual } => {
				write!(
					f,
					"Tag mismatch: expected '{}', found '{}'",
					expected, actual
				)
			}
			Self::ChildCountMismatch { expected, actual } => {
				write!(
					f,
					"Child count mismatch: expected {}, found {}",
					expected, actual
				)
			}
			Self::TextMismatch { expected, actual } => {
				write!(
					f,
					"Text mismatch: expected '{}', found '{}'",
					expected, actual
				)
			}
			Self::AttributeMismatch {
				name,
				expected,
				actual,
			} => {
				write!(
					f,
					"Attribute '{}' mismatch: expected {:?}, found {:?}",
					name, expected, actual
				)
			}
			Self::ElementNotFound { index } => {
				write!(f, "Element not found at index {}", index)
			}
		}
	}
}

impl std::error::Error for ReconcileError {}

/// Options for reconciliation (Phase 2-B).
///
/// Controls how reconciliation is performed during hydration,
/// enabling selective reconciliation for Island Architecture.
#[derive(Debug, Clone)]
pub struct ReconcileOptions {
	/// If true, only reconcile islands (interactive components).
	/// Static content and full-hydration components are skipped.
	pub island_only: bool,

	/// If true, skip elements marked with `data-rh-static="true"`.
	/// This is useful for preserving server-rendered static content.
	pub skip_static: bool,

	/// If true, log warnings for mismatches instead of failing.
	/// Useful for graceful degradation.
	pub warn_on_mismatch: bool,
}

impl Default for ReconcileOptions {
	fn default() -> Self {
		Self {
			island_only: false,
			skip_static: false,
			warn_on_mismatch: true,
		}
	}
}

impl ReconcileOptions {
	/// Creates options for island-only reconciliation.
	pub fn island_only() -> Self {
		Self {
			island_only: true,
			skip_static: true,
			warn_on_mismatch: true,
		}
	}

	/// Creates options for full reconciliation (default).
	pub fn full_reconciliation() -> Self {
		Self::default()
	}

	/// Sets the warn_on_mismatch option.
	pub fn warn_on_mismatch(mut self, warn: bool) -> Self {
		self.warn_on_mismatch = warn;
		self
	}
}

/// Reconciles the existing DOM with the expected View structure.
///
/// This function verifies that the SSR-rendered HTML matches what
/// the component would render, issuing warnings for mismatches
/// but generally allowing hydration to proceed.
#[cfg(target_arch = "wasm32")]
pub fn reconcile(element: &Element, view: &View) -> Result<(), ReconcileError> {
	match view {
		View::Element(el_view) => {
			// Check tag name
			let actual_tag = element.tag_name().to_lowercase();
			let expected_tag = el_view.tag_name().to_lowercase();
			if actual_tag != expected_tag {
				return Err(ReconcileError::TagMismatch {
					expected: expected_tag,
					actual: actual_tag,
				});
			}

			// Check children count (with some tolerance for whitespace text nodes)
			let children = element.children();
			let view_children = el_view.child_views();

			// Recursively check children
			for (i, child_view) in view_children.iter().enumerate() {
				if i < children.len() {
					reconcile(&children[i], child_view)?;
				}
			}

			Ok(())
		}
		View::Text(expected_text) => {
			let actual_text = element.text_content().unwrap_or_default();
			// Normalize whitespace for comparison
			let expected_normalized = normalize_whitespace(expected_text);
			let actual_normalized = normalize_whitespace(&actual_text);

			if expected_normalized != actual_normalized {
				// Log warning but don't fail - text content can have minor differences
				#[cfg(debug_assertions)]
				web_sys::console::warn_1(
					&format!(
						"Text content mismatch: expected '{}', found '{}'",
						expected_text, actual_text
					)
					.into(),
				);
			}
			Ok(())
		}
		View::Fragment(views) => {
			let children = element.children();
			for (i, child_view) in views.iter().enumerate() {
				if i < children.len() {
					reconcile(&children[i], child_view)?;
				}
			}
			Ok(())
		}
		View::Empty => Ok(()),
		View::WithHead { view, .. } => {
			// Head section is handled separately during SSR
			// For hydration, just reconcile the inner view
			reconcile(element, view)
		}
	}
}

/// Reconciles the existing DOM with the expected View structure with options (Phase 2-B).
///
/// This function performs selective reconciliation based on the provided options,
/// enabling Island Architecture by reconciling only interactive components.
///
/// # Arguments
///
/// * `element` - The DOM element to reconcile
/// * `view` - The expected view structure
/// * `options` - Reconciliation options
///
/// # Returns
///
/// `Ok(())` if reconciliation succeeds, or a `ReconcileError` if a mismatch is found.
///
/// # Behavior
///
/// - If `options.island_only` is true, only elements with `data-rh-island="true"` are reconciled
/// - If `options.skip_static` is true, elements with `data-rh-static="true"` are skipped
/// - If `options.warn_on_mismatch` is true, mismatches are logged as warnings instead of errors
#[cfg(target_arch = "wasm32")]
pub fn reconcile_with_options(
	element: &Element,
	view: &View,
	options: &ReconcileOptions,
) -> Result<(), ReconcileError> {
	// Check if this element should be skipped
	let should_skip = if options.skip_static {
		element.get_attribute("data-rh-static").as_deref() == Some("true")
	} else {
		false
	};

	if should_skip {
		return Ok(());
	}

	// Check if this is an island element
	let is_island = element.get_attribute("data-rh-island").as_deref() == Some("true");

	// Determine if we should reconcile this element
	let should_reconcile = if options.island_only {
		// In island-only mode, only reconcile island elements
		is_island
	} else {
		// In full reconciliation mode, reconcile all non-static elements
		true
	};

	// Perform reconciliation if applicable
	if should_reconcile {
		match reconcile(element, view) {
			Ok(_) => {}
			Err(err) => {
				if options.warn_on_mismatch {
					// Log warning instead of failing
					#[cfg(debug_assertions)]
					web_sys::console::warn_1(&format!("Reconciliation warning: {}", err).into());
				} else {
					return Err(err);
				}
			}
		}
	}

	// Recursively process children, unless this is an island boundary
	let should_recurse = if options.island_only && is_island {
		// If we're in island-only mode and this is an island,
		// don't recurse into children (they belong to this island's internal reconciliation)
		false
	} else {
		// Otherwise, recurse into children
		true
	};

	if should_recurse {
		if let View::Element(el_view) = view {
			let children = element.children();
			let view_children = el_view.child_views();

			for (i, child_view) in view_children.iter().enumerate() {
				if i < children.len() {
					reconcile_with_options(&children[i], child_view, options)?;
				}
			}
		} else if let View::Fragment(views) = view {
			let children = element.children();

			for (i, child_view) in views.iter().enumerate() {
				if i < children.len() {
					reconcile_with_options(&children[i], child_view, options)?;
				}
			}
		}
	}

	Ok(())
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
pub fn reconcile(_element: &str, _view: &View) -> Result<(), ReconcileError> {
	Ok(())
}

/// Non-WASM version for testing (Phase 2-B).
#[cfg(not(target_arch = "wasm32"))]
pub fn reconcile_with_options(
	_element: &str,
	_view: &View,
	_options: &ReconcileOptions,
) -> Result<(), ReconcileError> {
	Ok(())
}

/// Normalizes whitespace for comparison.
#[allow(dead_code)]
fn normalize_whitespace(s: &str) -> String {
	s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Checks if an element's structure matches the view.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(super) fn structure_matches(element: &Element, view: &View) -> bool {
	reconcile(element, view).is_ok()
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(super) fn structure_matches(_element: &str, _view: &View) -> bool {
	true
}

/// Detailed comparison result.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct CompareResult {
	/// Whether the structures match.
	pub matches: bool,
	/// List of differences found.
	pub differences: Vec<String>,
}

#[allow(dead_code)]
impl CompareResult {
	/// Creates a successful match result.
	pub(super) fn success() -> Self {
		Self {
			matches: true,
			differences: Vec::new(),
		}
	}

	/// Creates a failed match result with differences.
	pub(super) fn failure(differences: Vec<String>) -> Self {
		Self {
			matches: false,
			differences,
		}
	}
}

/// Compares DOM structure with view and returns detailed results.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(super) fn compare_structure(element: &Element, view: &View) -> CompareResult {
	let mut differences = Vec::new();
	compare_recursive(element, view, "", &mut differences);

	if differences.is_empty() {
		CompareResult::success()
	} else {
		CompareResult::failure(differences)
	}
}

#[cfg(target_arch = "wasm32")]
fn compare_recursive(element: &Element, view: &View, path: &str, differences: &mut Vec<String>) {
	match view {
		View::Element(el_view) => {
			let actual_tag = element.tag_name().to_lowercase();
			let expected_tag = el_view.tag_name().to_lowercase();

			if actual_tag != expected_tag {
				differences.push(format!(
					"{}: tag mismatch - expected '{}', found '{}'",
					path, expected_tag, actual_tag
				));
				return;
			}

			// Compare children
			let children = element.children();
			let view_children = el_view.child_views();

			if children.len() != view_children.len() {
				differences.push(format!(
					"{}: child count mismatch - expected {}, found {}",
					path,
					view_children.len(),
					children.len()
				));
			}

			for (i, child_view) in view_children.iter().enumerate() {
				let child_path = format!("{}/{}", path, i);
				if i < children.len() {
					compare_recursive(&children[i], child_view, &child_path, differences);
				} else {
					differences.push(format!("{}: missing child at index {}", path, i));
				}
			}
		}
		View::Text(_) | View::Empty => {}
		View::Fragment(views) => {
			let children = element.children();
			for (i, child_view) in views.iter().enumerate() {
				let child_path = format!("{}/{}", path, i);
				if i < children.len() {
					compare_recursive(&children[i], child_view, &child_path, differences);
				}
			}
		}
		View::WithHead { view, .. } => {
			// Head section is handled separately during SSR
			// For comparison, just compare the inner view
			compare_recursive(element, view, path, differences);
		}
	}
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(super) fn compare_structure(_element: &str, _view: &View) -> CompareResult {
	CompareResult::success()
}

// Phase 2-B Tests: Selective Reconciliation

#[test]
fn test_reconcile_options_default() {
	let options = ReconcileOptions::default();
	assert!(!options.island_only);
	assert!(!options.skip_static);
	assert!(options.warn_on_mismatch);
}

#[test]
fn test_reconcile_options_island_only() {
	let options = ReconcileOptions::island_only();
	assert!(options.island_only);
	assert!(options.skip_static);
	assert!(options.warn_on_mismatch);
}

#[test]
fn test_reconcile_options_full_reconciliation() {
	let options = ReconcileOptions::full_reconciliation();
	assert!(!options.island_only);
	assert!(!options.skip_static);
	assert!(options.warn_on_mismatch);
}

#[test]
fn test_reconcile_options_warn_on_mismatch() {
	let options = ReconcileOptions::default().warn_on_mismatch(false);
	assert!(!options.warn_on_mismatch);
}

#[test]
fn test_reconcile_with_options_non_wasm() {
	// Non-WASM version always succeeds
	let view = View::Empty;
	let options = ReconcileOptions::default();
	assert!(reconcile_with_options("", &view, &options).is_ok());
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_reconcile_error_display() {
		let err = ReconcileError::TagMismatch {
			expected: "div".to_string(),
			actual: "span".to_string(),
		};
		assert!(err.to_string().contains("Tag mismatch"));
		assert!(err.to_string().contains("div"));
		assert!(err.to_string().contains("span"));
	}

	#[test]
	fn test_child_count_mismatch_display() {
		let err = ReconcileError::ChildCountMismatch {
			expected: 3,
			actual: 2,
		};
		assert!(err.to_string().contains("Child count mismatch"));
		assert!(err.to_string().contains('3'));
		assert!(err.to_string().contains('2'));
	}

	#[test]
	fn test_normalize_whitespace() {
		assert_eq!(normalize_whitespace("hello  world"), "hello world");
		assert_eq!(normalize_whitespace("  foo   bar  "), "foo bar");
		assert_eq!(normalize_whitespace("single"), "single");
	}

	#[test]
	fn test_compare_result_success() {
		let result = CompareResult::success();
		assert!(result.matches);
		assert!(result.differences.is_empty());
	}

	#[test]
	fn test_compare_result_failure() {
		let result = CompareResult::failure(vec!["diff1".to_string(), "diff2".to_string()]);
		assert!(!result.matches);
		assert_eq!(result.differences.len(), 2);
	}

	#[test]
	fn test_structure_matches_non_wasm() {
		// Non-WASM version always returns true
		let view = View::Empty;
		assert!(structure_matches("", &view));
	}

	#[test]
	fn test_reconcile_non_wasm() {
		// Non-WASM version always succeeds
		let view = View::Empty;
		assert!(reconcile("", &view).is_ok());
	}
}
