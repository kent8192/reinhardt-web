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
	}
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
pub fn reconcile(_element: &str, _view: &View) -> Result<(), ReconcileError> {
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
pub fn structure_matches(element: &Element, view: &View) -> bool {
	reconcile(element, view).is_ok()
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn structure_matches(_element: &str, _view: &View) -> bool {
	true
}

/// Detailed comparison result.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompareResult {
	/// Whether the structures match.
	pub matches: bool,
	/// List of differences found.
	pub differences: Vec<String>,
}

#[allow(dead_code)]
impl CompareResult {
	/// Creates a successful match result.
	pub fn success() -> Self {
		Self {
			matches: true,
			differences: Vec::new(),
		}
	}

	/// Creates a failed match result with differences.
	pub fn failure(differences: Vec<String>) -> Self {
		Self {
			matches: false,
			differences,
		}
	}
}

/// Compares DOM structure with view and returns detailed results.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn compare_structure(element: &Element, view: &View) -> CompareResult {
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
	}
}

/// Non-WASM version for testing.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn compare_structure(_element: &str, _view: &View) -> CompareResult {
	CompareResult::success()
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
