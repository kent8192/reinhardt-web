//! Test utilities for page! macro components

use reinhardt_pages::component::View;

/// Evaluates a page! macro closure and returns the generated View
///
/// # Example
/// ```rust
/// let view = eval_page_component(page!(|| { button { "Test" } }));
/// assert!(view.render_to_string().contains("Test"));
/// ```
pub fn eval_page_component<F>(f: F) -> View
where
	F: FnOnce() -> View,
{
	f()
}

/// Evaluates a parameterized page! macro closure
///
/// # Example
/// ```rust
/// let view = eval_page_component_with_args(
///     page!(|name: &str| { div { {name} } }),
///     "Alice"
/// );
/// ```
pub fn eval_page_component_with_args<F, Args>(f: F, args: Args) -> View
where
	F: FnOnce(Args) -> View,
{
	f(args)
}

/// Checks if rendered HTML contains the specified text
pub fn contains_text(html: &str, text: &str) -> bool {
	html.contains(text)
}

/// Checks if rendered HTML contains the specified element
pub fn contains_element(html: &str, tag: &str) -> bool {
	html.contains(&format!("<{}", tag))
}

/// Checks if rendered HTML contains the specified attribute
pub fn contains_attribute(html: &str, name: &str, value: &str) -> bool {
	html.contains(&format!("{}=\"{}\"", name, value))
}
