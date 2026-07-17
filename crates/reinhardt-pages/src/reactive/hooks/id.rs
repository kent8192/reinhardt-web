//! ID hook: use_id
//!
//! This hook provides unique ID generation for accessibility and hydration.

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique IDs.
static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[cfg(native)]
tokio::task_local! {
	static SSR_ID_COUNTER: Rc<Cell<usize>>;
}

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
	let id = next_id();
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
	let id = next_id();
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
	#[cfg(native)]
	if let Ok(()) = SSR_ID_COUNTER.try_with(|counter| counter.set(0)) {
		return;
	}
	ID_COUNTER.store(0, Ordering::Relaxed);
}

#[doc(hidden)]
pub async fn scope_id_counter<R>(future: impl Future<Output = R>) -> R {
	scope_id_counter_with(Rc::new(Cell::new(0)), future).await
}

#[doc(hidden)]
pub async fn scope_id_counter_with<R>(
	counter: Rc<Cell<usize>>,
	future: impl Future<Output = R>,
) -> R {
	#[cfg(native)]
	{
		SSR_ID_COUNTER.scope(counter, future).await
	}
	#[cfg(not(native))]
	{
		let _ = counter;
		future.await
	}
}

#[doc(hidden)]
pub fn id_counter_snapshot() -> usize {
	#[cfg(native)]
	if let Ok(snapshot) = SSR_ID_COUNTER.try_with(|counter| counter.get()) {
		return snapshot;
	}
	ID_COUNTER.load(Ordering::Relaxed)
}

#[doc(hidden)]
pub fn restore_id_counter(snapshot: usize) {
	#[cfg(native)]
	if let Ok(()) = SSR_ID_COUNTER.try_with(|counter| counter.set(snapshot)) {
		return;
	}
	ID_COUNTER.store(snapshot, Ordering::Relaxed);
}

fn next_id() -> usize {
	#[cfg(native)]
	if let Ok(id) = SSR_ID_COUNTER.try_with(|counter| {
		let id = counter.get();
		counter.set(id + 1);
		id
	}) {
		return id;
	}
	ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_use_id_unique() {
		let id1 = use_id();
		let id2 = use_id();
		let id3 = use_id();

		assert_ne!(id1, id2);
		assert_ne!(id2, id3);
		assert_ne!(id1, id3);
	}

	#[test]
	fn test_use_id_format() {
		let id = use_id();
		assert!(id.starts_with("reinhardt-id-"));
	}

	#[test]
	fn test_use_id_with_prefix() {
		let id = use_id_with_prefix("custom");
		assert!(id.starts_with("custom-"));
	}

	#[test]
	fn test_use_id_sequential() {
		let id1 = use_id();
		let id2 = use_id();
		let first_id = id1
			.strip_prefix("reinhardt-id-")
			.expect("use_id returns the default prefix")
			.parse::<usize>()
			.expect("use_id returns a numeric suffix");

		assert_eq!(id2, format!("reinhardt-id-{}", first_id + 1));
	}
}
