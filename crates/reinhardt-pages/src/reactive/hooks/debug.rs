//! Debug hooks.
//!
//! These hooks provide debugging utilities.

/// Adds a label to a custom hook in DevTools.
///
/// This is the React-like equivalent of `useDebugValue`. It allows you to
/// display a custom label for your custom hooks in browser DevTools.
///
/// # Type Parameters
///
/// * `T` - The type of the debug value
///
/// # Arguments
///
/// * `value` - The value to display in DevTools
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_state, use_debug_value};
///
/// fn use_online_status() -> Signal<bool> {
///     let (is_online, set_online) = use_state(true);
///
///     // Show "Online" or "Offline" in DevTools
///     use_debug_value(if is_online.get() { "Online" } else { "Offline" });
///
///     is_online
/// }
/// ```
///
/// # Note
///
/// This is a no-op in production builds. It's only used for debugging purposes.
/// The actual DevTools integration would require browser-specific implementation.
pub fn use_debug_value<T: std::fmt::Debug>(_value: T) {
	// Note: Full DevTools integration (like React DevTools) would require a
	// dedicated browser extension. The current implementation logs values to
	// the console via debug_log! macro when debug-hooks feature is enabled.

	#[cfg(debug_assertions)]
	{
		crate::debug_log!("Debug value: {:?}", _value);
	}
}

/// Adds a label with formatting to a custom hook in DevTools.
///
/// This variant accepts a formatting function that is only called when
/// DevTools is attached, avoiding unnecessary formatting work.
///
/// # Arguments
///
/// * `value` - The raw value
/// * `format` - A function that formats the value for display
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_debug_value_with;
///
/// let items = use_state(vec![1, 2, 3, 4, 5]);
///
/// // Only format when DevTools is open
/// use_debug_value_with(&items.get(), |items| {
///     format!("Items: {} total", items.len())
/// });
/// ```
///
/// # Reactivity semantics
///
/// The `_format` closure runs outside any active reactive Observer.
/// Reading `Signal::get()`, `Memo::get()`, or `Resource::get()` inside
/// returns the latest value WITHOUT subscribing for future changes
/// (Option A, Refs #4195).
pub fn use_debug_value_with<T, F, R>(_value: T, _format: F)
where
	F: FnOnce(T) -> R,
	R: std::fmt::Debug,
{
	#[cfg(debug_assertions)]
	{
		let _formatted = _format(_value);
		crate::debug_log!("Debug value: {:?}", _formatted);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_use_debug_value() {
		// Should not panic
		use_debug_value(42);
		use_debug_value("test");
		use_debug_value(vec![1, 2, 3]);
	}

	#[test]
	fn test_use_debug_value_with() {
		// Should not panic
		use_debug_value_with(vec![1, 2, 3], |v| format!("len: {}", v.len()));
	}
}
