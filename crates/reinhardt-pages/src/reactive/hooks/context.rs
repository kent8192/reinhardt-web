//! Context hook: use_context
//!
//! This hook provides access to context values from the context system.

use crate::reactive::context::{Context, get_context};

/// Reads a value from the nearest ancestor context provider.
///
/// This is the React-like equivalent of `useContext`. It retrieves the value
/// provided by the nearest ancestor `ContextProvider` for the given context.
///
/// # Type Parameters
///
/// * `T` - The type of the context value
///
/// # Arguments
///
/// * `ctx` - The context to read from
///
/// # Returns
///
/// `Some(T)` if the context is provided, `None` otherwise
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::context::{create_context, provide_context};
/// use reinhardt_pages::reactive::hooks::use_context;
///
/// // Create a context for theme
/// static THEME_CONTEXT: Context<String> = create_context();
///
/// // In a parent component
/// provide_context(&THEME_CONTEXT, "dark".to_string());
///
/// // In a child component
/// fn child_component() -> View {
///     let theme = use_context(&THEME_CONTEXT)
///         .unwrap_or_else(|| "light".to_string());
///
///     page!(|| {
///         div {
///             class: format!("theme-{}", theme),
///             "Themed content"
///         }
///     })()
/// }
/// ```
///
/// # Note
///
/// Unlike React's useContext which throws if no provider is found, this
/// returns `Option<T>` to handle the case gracefully in Rust.
pub fn use_context<T: Clone + 'static>(ctx: &Context<T>) -> Option<T> {
	get_context(ctx)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::context::provide_context;
	use rstest::rstest;

	#[rstest]
	fn test_use_context_with_value() {
		let ctx: Context<i32> = Context::new();
		provide_context(&ctx, 42);

		let value = use_context(&ctx);
		assert_eq!(value, Some(42));
	}

	#[rstest]
	fn test_use_context_without_value() {
		let ctx: Context<String> = Context::new();

		let value = use_context(&ctx);
		assert!(value.is_none());
	}
}
