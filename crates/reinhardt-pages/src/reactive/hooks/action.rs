//! Action hooks: use_optimistic
//!
//! These hooks provide React-like optimistic update support.

use crate::reactive::Signal;

/// State for optimistic updates.
pub struct OptimisticState<T: 'static> {
	/// The current value (may be optimistic).
	value: Signal<T>,
	/// The actual confirmed value.
	confirmed: Signal<T>,
	/// Whether an optimistic update is pending.
	is_optimistic: Signal<bool>,
}

impl<T: Clone + 'static> OptimisticState<T> {
	/// Gets the current value (optimistic if pending, confirmed otherwise).
	pub fn get(&self) -> T {
		self.value.get()
	}

	/// Returns whether the current value is optimistic (unconfirmed).
	pub fn is_optimistic(&self) -> bool {
		self.is_optimistic.get()
	}

	/// Applies an optimistic update.
	pub fn update_optimistic(&self, value: T) {
		self.is_optimistic.set(true);
		self.value.set(value);
	}

	/// Confirms the value (called after successful async operation).
	pub fn confirm(&self, value: T) {
		self.confirmed.set(value.clone());
		self.value.set(value);
		self.is_optimistic.set(false);
	}

	/// Reverts to the confirmed value (called on error).
	pub fn revert(&self) {
		self.value.set(self.confirmed.get());
		self.is_optimistic.set(false);
	}
}

impl<T: Clone + 'static> Clone for OptimisticState<T> {
	fn clone(&self) -> Self {
		Self {
			value: self.value.clone(),
			confirmed: self.confirmed.clone(),
			is_optimistic: self.is_optimistic.clone(),
		}
	}
}

/// Enables optimistic UI updates during async operations.
///
/// This is the React-like equivalent of `useOptimistic`. It allows you to show
/// an optimistic (predicted) state while an async operation is pending, then
/// either confirm or revert based on the result.
///
/// # Type Parameters
///
/// * `T` - The type of the state value
///
/// # Arguments
///
/// * `initial` - The initial (confirmed) value
///
/// # Returns
///
/// An `OptimisticState<T>` for managing optimistic updates
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_optimistic;
///
/// let likes = use_optimistic(100);
///
/// let on_like = use_callback({
///     let likes = likes.clone();
///     move |_| {
///         // Optimistically increment
///         likes.update_optimistic(likes.get() + 1);
///
///         // Perform async operation
///         spawn_task({
///             let likes = likes.clone();
///             async move {
///                 match api_like_post().await {
///                     Ok(new_count) => likes.confirm(new_count),
///                     Err(_) => likes.revert(),
///                 }
///             }
///         });
///     }
/// });
///
/// page!(|| {
///     button {
///         @click: on_like,
///         format!("♥ {} {}", likes.get(), if likes.is_optimistic() { "(updating...)" } else { "" })
///     }
/// })
/// ```
pub fn use_optimistic<T: Clone + 'static>(initial: T) -> OptimisticState<T> {
	let value = Signal::new(initial.clone());
	let confirmed = Signal::new(initial);
	let is_optimistic = Signal::new(false);

	OptimisticState {
		value,
		confirmed,
		is_optimistic,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_use_optimistic_basic() {
		let state = use_optimistic(10);

		assert_eq!(state.get(), 10);
		assert!(!state.is_optimistic());

		// Apply optimistic update
		state.update_optimistic(20);
		assert_eq!(state.get(), 20);
		assert!(state.is_optimistic());

		// Confirm the update
		state.confirm(20);
		assert_eq!(state.get(), 20);
		assert!(!state.is_optimistic());
	}

	#[test]
	fn test_use_optimistic_revert() {
		let state = use_optimistic(10);

		// Apply optimistic update
		state.update_optimistic(20);
		assert_eq!(state.get(), 20);

		// Revert on error
		state.revert();
		assert_eq!(state.get(), 10);
		assert!(!state.is_optimistic());
	}

	#[test]
	fn test_optimistic_state_clone() {
		let state1 = use_optimistic(42);
		let state2 = state1.clone();

		state1.update_optimistic(100);
		assert_eq!(state2.get(), 100);
	}
}
