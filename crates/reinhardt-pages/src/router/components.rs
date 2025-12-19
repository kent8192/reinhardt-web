//! Router Components for navigation.
//!
//! This module provides Link and RouterOutlet components for
//! declarative navigation in component trees.

use crate::component::{Component, ElementView, IntoView, View};

/// A link component that navigates without full page reload.
///
/// Similar to HTML `<a>` but intercepts clicks to use the History API.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::router::Link;
///
/// let link = Link::new("/users/42/", "View User");
/// ```
#[derive(Debug, Clone)]
pub struct Link {
	/// The destination path.
	to: String,
	/// The link text or content.
	content: String,
	/// Additional CSS classes.
	class: Option<String>,
	/// Whether to replace the current history entry.
	replace: bool,
	/// Whether to open in a new tab (disables SPA navigation).
	external: bool,
	/// Custom attributes.
	attrs: Vec<(String, String)>,
}

impl Link {
	/// Creates a new link.
	pub fn new(to: impl Into<String>, content: impl Into<String>) -> Self {
		Self {
			to: to.into(),
			content: content.into(),
			class: None,
			replace: false,
			external: false,
			attrs: Vec::new(),
		}
	}

	/// Sets the CSS class.
	pub fn class(mut self, class: impl Into<String>) -> Self {
		self.class = Some(class.into());
		self
	}

	/// Sets whether to replace the current history entry.
	pub fn replace(mut self, replace: bool) -> Self {
		self.replace = replace;
		self
	}

	/// Sets whether this is an external link.
	pub fn external(mut self, external: bool) -> Self {
		self.external = external;
		self
	}

	/// Adds a custom attribute.
	pub fn attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.attrs.push((name.into(), value.into()));
		self
	}

	/// Returns the destination path.
	pub fn to(&self) -> &str {
		&self.to
	}

	/// Returns the content.
	pub fn content(&self) -> &str {
		&self.content
	}

	/// Returns whether this is a replace navigation.
	pub fn is_replace(&self) -> bool {
		self.replace
	}

	/// Returns whether this is an external link.
	pub fn is_external(&self) -> bool {
		self.external
	}
}

impl Component for Link {
	fn render(&self) -> View {
		let mut el = ElementView::new("a").attr("href", self.to.clone());

		if let Some(ref class) = self.class {
			el = el.attr("class", class.clone());
		}

		// Add data attributes for JS handling
		if !self.external {
			el = el.attr("data-link", "true");
			if self.replace {
				el = el.attr("data-replace", "true");
			}
		} else {
			el = el.attr("target", "_blank");
			el = el.attr("rel", "noopener noreferrer");
		}

		// Add custom attributes
		for (name, value) in &self.attrs {
			el = el.attr(name.clone(), value.clone());
		}

		el.child(self.content.clone()).into_view()
	}

	fn name() -> &'static str {
		"Link"
	}
}

/// A component that renders the matched route's content.
///
/// Place this where you want route content to appear.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::router::RouterOutlet;
///
/// let outlet = RouterOutlet::new();
/// ```
#[derive(Debug, Clone, Default)]
pub struct RouterOutlet {
	/// The ID attribute for the outlet element.
	id: Option<String>,
	/// CSS class for the outlet element.
	class: Option<String>,
}

impl RouterOutlet {
	/// Creates a new router outlet.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the ID attribute.
	pub fn id(mut self, id: impl Into<String>) -> Self {
		self.id = Some(id.into());
		self
	}

	/// Sets the CSS class.
	pub fn class(mut self, class: impl Into<String>) -> Self {
		self.class = Some(class.into());
		self
	}
}

impl Component for RouterOutlet {
	fn render(&self) -> View {
		let mut el = ElementView::new("div").attr("data-router-outlet", "true");

		if let Some(ref id) = self.id {
			el = el.attr("id", id.clone());
		}

		if let Some(ref class) = self.class {
			el = el.attr("class", class.clone());
		}

		// In actual implementation, this would be populated by the router
		// For now, render an empty placeholder
		el.into_view()
	}

	fn name() -> &'static str {
		"RouterOutlet"
	}
}

/// A redirect component that immediately navigates to another path.
#[derive(Debug, Clone)]
pub struct Redirect {
	/// The destination path.
	to: String,
	/// Whether to replace the current history entry.
	replace: bool,
}

impl Redirect {
	/// Creates a new redirect.
	pub fn new(to: impl Into<String>) -> Self {
		Self {
			to: to.into(),
			replace: true,
		}
	}

	/// Sets whether to use replace navigation.
	pub fn replace(mut self, replace: bool) -> Self {
		self.replace = replace;
		self
	}

	/// Returns the destination path.
	pub fn to(&self) -> &str {
		&self.to
	}
}

impl Component for Redirect {
	fn render(&self) -> View {
		// Render a meta refresh as fallback, actual redirect handled by JS
		ElementView::new("meta")
			.attr("http-equiv", "refresh")
			.attr("content", format!("0;url={}", self.to))
			.attr("data-redirect", self.to.clone())
			.attr("data-replace", if self.replace { "true" } else { "false" })
			.into_view()
	}

	fn name() -> &'static str {
		"Redirect"
	}
}

/// A navigation guard that conditionally renders content.
///
/// This is a function that wraps content rendering with a condition check.
pub fn guard<F, V>(condition: F, content: V) -> impl FnOnce() -> View
where
	F: FnOnce() -> bool,
	V: IntoView,
{
	move || {
		if condition() {
			content.into_view()
		} else {
			View::Empty
		}
	}
}

/// A navigation guard with fallback that conditionally renders content.
pub fn guard_or<F, V, U>(condition: F, content: V, fallback: U) -> impl FnOnce() -> View
where
	F: FnOnce() -> bool,
	V: IntoView,
	U: IntoView,
{
	move || {
		if condition() {
			content.into_view()
		} else {
			fallback.into_view()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_link_new() {
		let link = Link::new("/users/", "Users");
		assert_eq!(link.to(), "/users/");
		assert_eq!(link.content(), "Users");
		assert!(!link.is_replace());
		assert!(!link.is_external());
	}

	#[test]
	fn test_link_builder() {
		let link = Link::new("/admin/", "Admin")
			.class("nav-link")
			.replace(true)
			.attr("aria-label", "Admin Panel");

		let html = link.render().render_to_string();
		assert!(html.contains("href=\"/admin/\""));
		assert!(html.contains("class=\"nav-link\""));
		assert!(html.contains("data-replace=\"true\""));
		assert!(html.contains("aria-label=\"Admin Panel\""));
	}

	#[test]
	fn test_link_external() {
		let link = Link::new("https://example.com", "Example").external(true);

		let html = link.render().render_to_string();
		assert!(html.contains("target=\"_blank\""));
		assert!(html.contains("rel=\"noopener noreferrer\""));
		assert!(!html.contains("data-link"));
	}

	#[test]
	fn test_router_outlet() {
		let outlet = RouterOutlet::new().id("main-outlet").class("content");

		let html = outlet.render().render_to_string();
		assert!(html.contains("data-router-outlet=\"true\""));
		assert!(html.contains("id=\"main-outlet\""));
		assert!(html.contains("class=\"content\""));
	}

	#[test]
	fn test_redirect() {
		let redirect = Redirect::new("/login/");
		assert_eq!(redirect.to(), "/login/");

		let html = redirect.render().render_to_string();
		assert!(html.contains("url=/login/"));
		assert!(html.contains("data-redirect=\"/login/\""));
	}

	#[test]
	fn test_guard_true() {
		let view = guard(|| true, "Allowed")();
		assert_eq!(view.render_to_string(), "Allowed");
	}

	#[test]
	fn test_guard_false() {
		let view = guard(|| false, "Allowed")();
		assert_eq!(view.render_to_string(), "");
	}

	#[test]
	fn test_guard_with_fallback() {
		let view = guard_or(|| false, "Allowed", "Denied")();
		assert_eq!(view.render_to_string(), "Denied");
	}

	#[test]
	fn test_link_component_name() {
		assert_eq!(Link::name(), "Link");
	}

	#[test]
	fn test_router_outlet_component_name() {
		assert_eq!(RouterOutlet::name(), "RouterOutlet");
	}

	#[test]
	fn test_redirect_component_name() {
		assert_eq!(Redirect::name(), "Redirect");
	}
}
