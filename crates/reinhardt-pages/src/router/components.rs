//! Router Components for navigation.
//!
//! This module provides Link and RouterOutlet components for
//! declarative navigation in component trees.

use crate::component::{Component, Head, IntoPage, MetaTag, Page, PageElement};
use crate::reactive::Signal;
use crate::router::loader::{LoaderStore, RouteLoaderError, with_loader_store};
use reinhardt_urls::routers::ClientRouter;
use std::borrow::Cow;
use std::rc::Rc;

/// Controls when a link prepares matched route loaders in the background.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrefetchMode {
	/// Do not prefetch route data.
	#[default]
	None,
	/// Prefetch on pointer intent or keyboard focus.
	Hover,
	/// Prefetch when the link enters the viewport.
	Viewport,
}

/// Controls how a [`Link`] decides whether it points at the current route.
///
/// Both modes compare only the path portion: a `?query` or `#fragment`
/// suffix on the link destination or on the current path is ignored.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ActiveMatch {
	/// The link is active only when its path equals the current path.
	///
	/// Trailing slashes are significant: `/account` does not match
	/// `/account/`.
	#[default]
	Exact,
	/// The link is active when its path equals the current path or is a
	/// leading path-segment prefix of it.
	///
	/// `/account/` matches `/account/` and `/account/settings/`, and
	/// `/account` matches `/account/settings/`, but `/acc` does not match
	/// `/account/`. A link to `/` matches every path.
	Prefix,
}

/// A link component that navigates without full page reload.
///
/// Similar to HTML `<a>` but intercepts clicks to use the History API.
///
/// # Active links
///
/// When an SPA router is installed (inside `ClientLauncher::launch()`), a
/// non-external link whose destination is a root-relative path (starting with
/// a single `/`) tracks the current route reactively:
///
/// - it renders `aria-current="page"` while it matches the current path,
///   unless an explicit `aria-current` attribute was set with [`Link::attr`];
/// - with [`Link::active_class`], the active class is appended to the
///   [`Link::class`] value while it matches, unless an explicit `class`
///   attribute was set with [`Link::attr`].
///
/// Matching is exact by default; [`Link::match_prefix`] switches to
/// path-segment prefix matching (see [`ActiveMatch`]). Without an installed
/// router, for example during server-side rendering, the link renders only
/// its static attributes, so active state never appears in SSR output.
///
/// Active-link rendering has P1 (symbol parity) on native targets, where
/// `ClientLauncher::launch()` never installs a router; the builders are
/// available on both targets.
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::router::Link;
///
/// let link = Link::new("/users/42/", "View User");
/// ```
///
/// A `#[layout]` shell stays mounted while its child routes change, so its
/// navigation uses active links to mark the current page:
///
/// ```rust
/// use reinhardt_pages::component::{Component, IntoPage, Page, PageElement};
/// use reinhardt_pages::router::Link;
///
/// fn shell_nav() -> Page {
///     PageElement::new("nav")
///         .child(Link::new("/", "Overview").class("nav-link").active_class("active").render())
///         .child(
///             Link::new("/account/", "Account")
///                 .class("nav-link")
///                 .active_class("active")
///                 .match_prefix()
///                 .render(),
///         )
///         .into_page()
/// }
///
/// // Without an installed SPA router the links render statically.
/// assert_eq!(
///     shell_nav().render_to_string(),
///     "<nav><a href=\"/\" class=\"nav-link\" data-link=\"true\">Overview</a>\
///      <a href=\"/account/\" class=\"nav-link\" data-link=\"true\">Account</a></nav>",
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Link {
	/// The destination path.
	to: String,
	/// The link text or content.
	content: String,
	/// Additional CSS classes.
	class: Option<String>,
	/// CSS class appended while the link matches the current route.
	active_class: Option<String>,
	/// How the destination is compared with the current route.
	active_match: ActiveMatch,
	/// Whether to replace the current history entry.
	replace: bool,
	/// Whether to open in a new tab (disables SPA navigation).
	external: bool,
	/// Background route-loader preparation policy.
	prefetch: PrefetchMode,
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
			active_class: None,
			active_match: ActiveMatch::Exact,
			replace: false,
			external: false,
			prefetch: PrefetchMode::None,
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

	/// Sets the route-loader prefetch policy for this link.
	pub fn prefetch(mut self, mode: PrefetchMode) -> Self {
		self.prefetch = mode;
		self
	}

	/// Sets the CSS class appended while the link matches the current route.
	///
	/// The class is added after the [`Link::class`] value, separated by a
	/// space, and removed again when a navigation moves away from the link's
	/// destination. It only takes effect when an SPA router is installed; see
	/// the [active links](Link#active-links) section.
	///
	/// An explicit `class` attribute set with [`Link::attr`] takes precedence:
	/// the link then keeps that static value and the active class is not
	/// applied, mirroring how an explicit `aria-current` attribute replaces
	/// the automatic one.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_pages::router::Link;
	///
	/// let link = Link::new("/account/", "Account")
	///     .class("nav-link")
	///     .active_class("active");
	/// assert_eq!(link.to(), "/account/");
	/// ```
	pub fn active_class(mut self, class: impl Into<String>) -> Self {
		self.active_class = Some(class.into());
		self
	}

	/// Marks the link active for every path below its destination.
	///
	/// Switches matching from [`ActiveMatch::Exact`] to
	/// [`ActiveMatch::Prefix`], which suits section links in a layout
	/// navigation.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_pages::router::{ActiveMatch, Link};
	///
	/// let link = Link::new("/account/", "Account").match_prefix();
	/// assert_eq!(link.active_match(), ActiveMatch::Prefix);
	/// ```
	pub fn match_prefix(mut self) -> Self {
		self.active_match = ActiveMatch::Prefix;
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

	/// Returns the configured prefetch mode.
	pub fn prefetch_mode(&self) -> PrefetchMode {
		self.prefetch
	}

	/// Returns how the link is compared with the current route.
	pub fn active_match(&self) -> ActiveMatch {
		self.active_match
	}

	/// Returns the installed router's path signal when this link takes part
	/// in active-link tracking.
	fn active_path_source(&self) -> Option<Signal<String>> {
		if self.external || !self.to.starts_with('/') || self.to.starts_with("//") {
			return None;
		}
		crate::reactive::hooks::use_router().current_path().ok()
	}

	/// Returns whether `name` was set explicitly through [`Link::attr`].
	fn has_explicit_attr(&self, name: &str) -> bool {
		self.attrs
			.iter()
			.any(|(attr_name, _)| attr_name.eq_ignore_ascii_case(name))
	}
}

/// Path comparison used for active-link state.
#[derive(Clone, Debug)]
struct ActiveTarget {
	path: String,
	mode: ActiveMatch,
}

impl ActiveTarget {
	fn new(to: &str, mode: ActiveMatch) -> Self {
		Self {
			path: path_component(to).to_owned(),
			mode,
		}
	}

	fn matches(&self, current: &str) -> bool {
		let current = path_component(current);
		match self.mode {
			ActiveMatch::Exact => current == self.path,
			ActiveMatch::Prefix => current
				.strip_prefix(self.path.as_str())
				.is_some_and(|rest| {
					rest.is_empty() || self.path.ends_with('/') || rest.starts_with('/')
				}),
		}
	}
}

/// Returns `url` without its `?query` and `#fragment` suffixes.
fn path_component(url: &str) -> &str {
	let url = url.split_once('#').map_or(url, |(path, _)| path);
	url.split_once('?').map_or(url, |(path, _)| path)
}

impl Component for Link {
	fn render(&self) -> Page {
		let mut el = PageElement::new("a").attr("href", self.to.clone());
		let active = self
			.active_path_source()
			.map(|current_path| (current_path, ActiveTarget::new(&self.to, self.active_match)));

		// Explicit `attr("class", ..)` / `attr("aria-current", ..)` values take
		// precedence over the route-derived reactive attributes.
		let reactive_class = if self.has_explicit_attr("class") {
			None
		} else {
			self.active_class.as_ref()
		};
		match (&active, reactive_class) {
			(Some((current_path, target)), Some(active_class)) => {
				let current_path = *current_path;
				let target = target.clone();
				let base_class = self.class.clone();
				let active_class = active_class.clone();
				el = el.reactive_attr("class", move || {
					let is_active = target.matches(&current_path.get());
					let class = match (base_class.as_deref(), is_active) {
						(Some(base), true) => format!("{base} {active_class}"),
						(None, true) => active_class.clone(),
						(Some(base), false) => base.to_owned(),
						(None, false) => return None,
					};
					Some(Cow::Owned(class))
				});
			}
			_ => {
				if let Some(ref class) = self.class {
					el = el.attr("class", class.clone());
				}
			}
		}

		// Add data attributes for JS handling
		if !self.external {
			el = el.attr("data-link", "true");
			if self.replace {
				el = el.attr("data-replace", "true");
			}
			match self.prefetch {
				PrefetchMode::None => {}
				PrefetchMode::Hover => el = el.attr("data-prefetch", "hover"),
				PrefetchMode::Viewport => el = el.attr("data-prefetch", "viewport"),
			}
		} else {
			el = el.attr("target", "_blank");
			el = el.attr("rel", "noopener noreferrer");
		}

		// Add custom attributes
		for (name, value) in &self.attrs {
			el = el.attr(name.clone(), value.clone());
		}

		if let Some((current_path, target)) = active
			&& !self.has_explicit_attr("aria-current")
		{
			el = el.reactive_attr("aria-current", move || {
				target
					.matches(&current_path.get())
					.then_some(Cow::Borrowed("page"))
			});
		}

		el.child(self.content.clone()).into_page()
	}

	fn name() -> &'static str {
		"Link"
	}
}

type NavigationErrorFallback = Rc<dyn Fn(&RouteLoaderError) -> Page>;

/// Renders the current route from a [`ClientRouter`].
///
/// `RouterOutlet` is a component-level adapter for embedding the canonical
/// `reinhardt-urls` client router in a pages component tree.
#[derive(Clone)]
pub struct RouterOutlet {
	/// Router used to resolve and render the current client-side route.
	router: ClientRouter,
	/// Optional sibling fallback for a failed route-loader navigation.
	navigation_error_fallback: Option<NavigationErrorFallback>,
}

impl std::fmt::Debug for RouterOutlet {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_struct("RouterOutlet")
			.field("router", &self.router)
			.field(
				"has_navigation_error_fallback",
				&self.navigation_error_fallback.is_some(),
			)
			.finish()
	}
}

impl RouterOutlet {
	/// Creates a new router outlet backed by `router`.
	pub fn new(router: ClientRouter) -> Self {
		Self {
			router,
			navigation_error_fallback: None,
		}
	}

	/// Adds a sibling boundary for errors raised while preparing a route.
	///
	/// The current route remains mounted; the fallback is rendered alongside it
	/// until a subsequent navigation clears the coordinator error.
	pub fn navigation_error_fallback<F>(mut self, fallback: F) -> Self
	where
		F: Fn(&RouteLoaderError) -> Page + 'static,
	{
		self.navigation_error_fallback = Some(Rc::new(fallback));
		self
	}

	/// Returns the router backing this outlet.
	pub fn router(&self) -> &ClientRouter {
		&self.router
	}
}

impl Component for RouterOutlet {
	fn render(&self) -> Page {
		let router = self.router.clone();
		let fallback = self.navigation_error_fallback.clone();
		Page::reactive(move || {
			let mounted_store = crate::app::try_with_navigation_coordinator(|coordinator| {
				coordinator.mounted_store()
			})
			.flatten();
			let current = render_current_with_mounted_loader_store(&router, mounted_store);
			if let Some(fallback) = &fallback
				&& let Some(Some(error)) =
					crate::app::try_with_navigation_coordinator(|coordinator| {
						coordinator.error().get()
					}) {
				return Page::Fragment(vec![current, fallback(&error)]);
			}
			current
		})
	}

	fn name() -> &'static str {
		"RouterOutlet"
	}
}

fn render_current_with_mounted_loader_store(
	router: &ClientRouter,
	mounted_store: Option<LoaderStore>,
) -> Page {
	if let Some(store) = mounted_store {
		return with_loader_store(&store, || router.render_current());
	}
	router.render_current()
}

impl IntoPage for RouterOutlet {
	fn into_page(self) -> Page {
		self.render()
	}
}

/// A redirect component that immediately navigates to another path.
///
/// The redirect URL is validated at construction time to prevent open redirect
/// attacks. Relative URLs (starting with `/`) are always allowed. Absolute URLs
/// must have their host in the provided `allowed_hosts` set.
#[derive(Debug, Clone)]
pub struct Redirect {
	/// The destination path.
	to: String,
	/// Whether to replace the current history entry.
	replace: bool,
}

impl Redirect {
	/// Creates a new redirect with URL validation.
	///
	/// Validates the redirect URL against the provided allowed hosts to prevent
	/// open redirect attacks. Relative URLs (starting with `/`) are always safe.
	///
	/// # Errors
	///
	/// Returns `RedirectValidationError` if the URL fails validation.
	pub fn validated(
		to: impl Into<String>,
		allowed_hosts: &std::collections::HashSet<String>,
	) -> Result<Self, reinhardt_core::security::redirect::RedirectValidationError> {
		let url = to.into();
		reinhardt_core::security::redirect::validate_redirect_url(&url, allowed_hosts)?;
		Ok(Self {
			to: url,
			replace: true,
		})
	}

	/// Creates a new redirect without external URL validation.
	///
	/// Only allows relative URLs (starting with `/`). Rejects any URL that
	/// could be an absolute URL or uses dangerous protocols.
	///
	/// # Panics
	///
	/// Panics if the URL does not start with `/` (use `validated` for absolute URLs).
	pub fn new(to: impl Into<String>) -> Self {
		let url = to.into();
		assert!(
			url.starts_with('/') && !url.starts_with("//"),
			"Redirect::new only accepts relative URLs starting with '/'. Use Redirect::validated for absolute URLs."
		);
		Self {
			to: url,
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
	fn render(&self) -> Page {
		let head = Head::new().meta(MetaTag::http_equiv("refresh", format!("0;url={}", self.to)));
		PageElement::new("span")
			.attr("data-redirect", self.to.clone())
			.attr("data-replace", if self.replace { "true" } else { "false" })
			.attr("hidden", "hidden")
			.into_page()
			.with_head(head)
	}

	fn name() -> &'static str {
		"Redirect"
	}
}

/// A navigation guard that conditionally renders content.
///
/// This is a function that wraps content rendering with a condition check.
pub fn guard<F, V>(condition: F, content: V) -> impl FnOnce() -> Page
where
	F: FnOnce() -> bool,
	V: IntoPage,
{
	move || {
		if condition() {
			content.into_page()
		} else {
			Page::Empty
		}
	}
}

/// A navigation guard with fallback that conditionally renders content.
pub fn guard_or<F, V, U>(condition: F, content: V, fallback: U) -> impl FnOnce() -> Page
where
	F: FnOnce() -> bool,
	V: IntoPage,
	U: IntoPage,
{
	move || {
		if condition() {
			content.into_page()
		} else {
			fallback.into_page()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::hooks::use_router;
	use crate::router::loader::active_loader_store;
	use reinhardt_core::reactive::ReactiveScope;
	use reinhardt_urls::routers::ClientRouter;
	use rstest::rstest;
	use serial_test::serial;

	/// Installs a client router in the thread-local SPA slot and clears the
	/// slot on drop, so a failing assertion cannot leak the router into a
	/// later test on the same thread.
	struct InstalledRouter;

	impl InstalledRouter {
		fn install(router: ClientRouter) -> Self {
			crate::app::__install_client_router_for_test(router);
			Self
		}
	}

	impl Drop for InstalledRouter {
		fn drop(&mut self) {
			crate::app::__clear_spa_router_for_test();
		}
	}

	/// Layout-shell routes; the native history stub starts every router at `/`.
	fn shell_router() -> ClientRouter {
		ClientRouter::new()
			.route("overview", "/", || Page::text("Overview"))
			.route("account", "/account/", || Page::text("Account"))
			.route("account-settings", "/account/settings/", || {
				Page::text("Settings")
			})
	}

	#[test]
	#[serial]
	fn router_outlet_reacts_to_loader_errors_after_its_initial_render() {
		ReactiveScope::run(|| {
			let router = ClientRouter::new().route("home", "/", || {
				assert!(
					active_loader_store().is_some(),
					"loader-backed rerenders must retain their mounted store"
				);
				Page::text("HOME")
			});
			crate::app::__install_client_router_for_test(router.clone());
			crate::app::try_with_navigation_coordinator(|coordinator| {
				coordinator.set_mounted_store_for_test(LoaderStore::new());
			})
			.expect("test router installs a navigation coordinator");
			let outlet = RouterOutlet::new(router).navigation_error_fallback(|error| {
				Page::text(format!("FAILED: {}", error.public_message()))
			});
			let page = outlet.render();

			assert_eq!(page.render_to_string(), "HOME");
			crate::app::try_with_navigation_coordinator(|coordinator| {
				coordinator
					.error()
					.set(Some(RouteLoaderError::new("loader failed")));
			})
			.expect("test router installs a navigation coordinator");
			assert_eq!(page.render_to_string(), "HOMEFAILED: loader failed");

			crate::app::__clear_spa_router_for_test();
		});
	}

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
	fn test_redirect() {
		let redirect = Redirect::new("/login/");
		assert_eq!(redirect.to(), "/login/");

		let html = redirect.render().render_to_string();
		assert!(html.contains("data-redirect=\"/login/\""));
		let page = redirect.render();
		assert!(
			page.extract_head()
				.expect("redirect fallback head")
				.to_html()
				.contains("url=/login/")
		);
	}

	#[test]
	#[should_panic(expected = "Redirect::new only accepts relative URLs")]
	fn test_redirect_rejects_absolute_url() {
		Redirect::new("https://evil.com/phish");
	}

	#[test]
	#[should_panic(expected = "Redirect::new only accepts relative URLs")]
	fn test_redirect_rejects_protocol_relative() {
		Redirect::new("//evil.com/path");
	}

	#[test]
	fn test_redirect_validated_allows_trusted_host() {
		let mut hosts = std::collections::HashSet::new();
		hosts.insert("example.com".to_string());
		let redirect = Redirect::validated("https://example.com/page", &hosts).unwrap();
		assert_eq!(redirect.to(), "https://example.com/page");
	}

	#[test]
	fn test_redirect_validated_rejects_untrusted_host() {
		let hosts = std::collections::HashSet::new();
		let result = Redirect::validated("https://evil.com/phish", &hosts);
		assert!(result.is_err());
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
	fn test_link_prefetch_attribute() {
		let html = Link::new("/jobs/", "Jobs")
			.prefetch(PrefetchMode::Hover)
			.render()
			.render_to_string();
		assert!(html.contains("data-prefetch=\"hover\""));
	}

	#[test]
	fn test_redirect_component_name() {
		assert_eq!(Redirect::name(), "Redirect");
	}

	#[rstest]
	fn link_active_match_defaults_to_exact_and_switches_with_match_prefix() {
		// Arrange
		let link = Link::new("/account/", "Account").active_class("active");

		// Act
		let prefixed = link.clone().match_prefix();

		// Assert
		assert_eq!(link.active_match(), ActiveMatch::Exact);
		assert_eq!(prefixed.active_match(), ActiveMatch::Prefix);
		assert_eq!(ActiveMatch::default(), ActiveMatch::Exact);
	}

	#[rstest]
	#[case::exact_equal("/account/", "/account/", ActiveMatch::Exact, true)]
	#[case::exact_child("/account/", "/account/settings/", ActiveMatch::Exact, false)]
	#[case::exact_trailing_slash("/account", "/account/", ActiveMatch::Exact, false)]
	#[case::exact_ignores_current_query(
		"/account/",
		"/account/?tab=keys",
		ActiveMatch::Exact,
		true
	)]
	#[case::exact_ignores_link_fragment("/account/#keys", "/account/", ActiveMatch::Exact, true)]
	#[case::exact_root_on_child("/", "/account/", ActiveMatch::Exact, false)]
	#[case::prefix_equal("/account/", "/account/", ActiveMatch::Prefix, true)]
	#[case::prefix_child("/account/", "/account/settings/", ActiveMatch::Prefix, true)]
	#[case::prefix_child_of_bare_segment(
		"/account",
		"/account/settings/",
		ActiveMatch::Prefix,
		true
	)]
	#[case::prefix_partial_segment("/acc", "/account/", ActiveMatch::Prefix, false)]
	#[case::prefix_sibling_segment("/account", "/accounts/", ActiveMatch::Prefix, false)]
	#[case::prefix_parent("/account/settings/", "/account/", ActiveMatch::Prefix, false)]
	#[case::prefix_root_on_child("/", "/account/", ActiveMatch::Prefix, true)]
	fn active_target_compares_path_components(
		#[case] to: &str,
		#[case] current: &str,
		#[case] mode: ActiveMatch,
		#[case] expected: bool,
	) {
		// Arrange
		let target = ActiveTarget::new(to, mode);

		// Act
		let matches = target.matches(current);

		// Assert
		assert_eq!(matches, expected);
	}

	#[rstest]
	#[serial(router)]
	fn link_without_router_renders_only_static_attributes() {
		// Arrange
		crate::app::__clear_spa_router_for_test();
		let link = Link::new("/", "Overview")
			.class("nav")
			.active_class("active");

		// Act
		let html = link.render().render_to_string();

		// Assert
		assert_eq!(
			html,
			"<a href=\"/\" class=\"nav\" data-link=\"true\">Overview</a>"
		);
	}

	#[rstest]
	#[case::active_with_both_classes(
		Link::new("/", "Overview").class("nav").active_class("active"),
		"<a href=\"/\" data-link=\"true\" class=\"nav active\" aria-current=\"page\">Overview</a>"
	)]
	#[case::inactive_with_both_classes(
		Link::new("/account/", "Account").class("nav").active_class("active"),
		"<a href=\"/account/\" data-link=\"true\" class=\"nav\">Account</a>"
	)]
	#[case::active_with_active_class_only(
		Link::new("/", "Overview").active_class("active"),
		"<a href=\"/\" data-link=\"true\" class=\"active\" aria-current=\"page\">Overview</a>"
	)]
	#[case::inactive_with_active_class_only(
		Link::new("/account/", "Account").active_class("active"),
		"<a href=\"/account/\" data-link=\"true\">Account</a>"
	)]
	#[case::active_without_active_class(
		Link::new("/", "Overview").class("nav"),
		"<a href=\"/\" class=\"nav\" data-link=\"true\" aria-current=\"page\">Overview</a>"
	)]
	#[case::explicit_aria_current_wins(
		Link::new("/", "Step").attr("aria-current", "step"),
		"<a href=\"/\" data-link=\"true\" aria-current=\"step\">Step</a>"
	)]
	#[case::explicit_class_attr_wins(
		Link::new("/", "Overview").active_class("active").attr("class", "custom"),
		"<a href=\"/\" data-link=\"true\" class=\"custom\" aria-current=\"page\">Overview</a>"
	)]
	#[case::external_link_is_never_active(
		Link::new("/", "Mirror").external(true).active_class("active"),
		"<a href=\"/\" target=\"_blank\" rel=\"noopener noreferrer\">Mirror</a>"
	)]
	#[case::protocol_relative_link_is_never_active(
		Link::new("//cdn.example.com/", "CDN").active_class("active"),
		"<a href=\"//cdn.example.com/\" data-link=\"true\">CDN</a>"
	)]
	#[serial(router)]
	fn link_with_router_reflects_the_current_route(#[case] link: Link, #[case] expected: &str) {
		ReactiveScope::run(|| {
			// Arrange
			let _router = InstalledRouter::install(shell_router());

			// Act
			let html = link.render().render_to_string();

			// Assert
			assert_eq!(html, expected);
		});
	}

	#[rstest]
	#[serial(router)]
	fn link_active_state_follows_navigation() {
		ReactiveScope::run(|| {
			// Arrange
			let _router = InstalledRouter::install(shell_router());
			let page = Link::new("/account/", "Account")
				.class("nav")
				.active_class("active")
				.match_prefix()
				.render();
			let before = page.render_to_string();

			// Act
			use_router()
				.push("/account/settings/")
				.expect("push to a registered route");
			let inside_section = page.render_to_string();
			use_router().push("/").expect("push to a registered route");
			let after_leaving = page.render_to_string();

			// Assert
			let inactive = "<a href=\"/account/\" data-link=\"true\" class=\"nav\">Account</a>";
			assert_eq!(before, inactive);
			assert_eq!(
				inside_section,
				"<a href=\"/account/\" data-link=\"true\" class=\"nav active\" aria-current=\"page\">Account</a>"
			);
			assert_eq!(after_leaving, inactive);
		});
	}
}
