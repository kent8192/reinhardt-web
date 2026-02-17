//! ID hook: use_id
//!
//! This hook provides unique ID generation for accessibility and hydration.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique IDs.
static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generates a unique ID that is stable across server and client.
///
/// This is the React-like equivalent of `useId`. It generates a unique string ID
/// that can be used for accessibility attributes like `aria-describedby` or
/// for associating labels with form controls.
///
/// # Returns
///
/// A unique string ID in the format `reinhardt-id-{counter}`
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_id;
/// use reinhardt_pages::page;
///
/// fn form_field(label: &str) -> View {
///     let id = use_id();
///
///     page!(|| {
///         div {
///             label {
///                 r#for: id.clone(),
///                 label
///             }
///             input {
///                 id: id,
///                 r#type: "text",
///             }
///         }
///     })()
/// }
/// ```
///
/// # Accessibility
///
/// Use this hook when you need to generate IDs for:
/// - Associating labels with form controls (`for` and `id` attributes)
/// - ARIA relationships (`aria-labelledby`, `aria-describedby`)
/// - Connecting tooltips or popovers to their triggers
///
/// # Note on Hydration
///
/// The generated IDs are deterministic based on the order of `use_id` calls.
/// This ensures that server-rendered HTML matches the client hydration,
/// preventing hydration mismatches.
///
/// For true hydration stability, consider using a seed-based approach
/// that takes the component tree position into account.
pub fn use_id() -> String {
	let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
	format!("reinhardt-id-{}", id)
}

/// Generates a unique ID with a custom prefix.
///
/// # Arguments
///
/// * `prefix` - The prefix to use instead of "reinhardt-id"
///
/// # Returns
///
/// A unique string ID in the format `{prefix}-{counter}`
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_id_with_prefix;
///
/// let id = use_id_with_prefix("modal");
/// // Returns something like "modal-42"
/// ```
pub fn use_id_with_prefix(prefix: &str) -> String {
	let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
	format!("{}-{}", prefix, id)
}

/// Resets the ID counter.
///
/// This is primarily useful for testing to ensure consistent IDs across test runs.
/// In production, this should generally not be called.
///
/// # Warning
///
/// Calling this in production code may cause ID collisions and hydration mismatches.
#[doc(hidden)]
pub fn reset_id_counter() {
	ID_COUNTER.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_use_id_unique() {
		reset_id_counter();

		let id1 = use_id();
		let id2 = use_id();
		let id3 = use_id();

		assert_ne!(id1, id2);
		assert_ne!(id2, id3);
		assert_ne!(id1, id3);
	}

	#[rstest]
	fn test_use_id_format() {
		reset_id_counter();

		let id = use_id();
		assert!(id.starts_with("reinhardt-id-"));
	}

	#[rstest]
	fn test_use_id_with_prefix() {
		reset_id_counter();

		let id = use_id_with_prefix("custom");
		assert!(id.starts_with("custom-"));
	}

	#[rstest]
	fn test_use_id_sequential() {
		reset_id_counter();

		let id1 = use_id();
		let id2 = use_id();

		assert_eq!(id1, "reinhardt-id-0");
		assert_eq!(id2, "reinhardt-id-1");
	}
}
