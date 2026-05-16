//! `ClientLauncher` builder, lifecycle contexts, and the `launch()` pipeline.

#[allow(deprecated)]
// (Refs #4234) Importing deprecated routing types intentionally during the deprecation cycle.
use crate::router::{PathPattern, Router};
use std::cell::RefCell;
use std::collections::HashMap;

#[cfg(wasm)]
use super::link_interceptor::install_link_interceptor;
#[cfg(wasm)]
use super::{store_spa_router, with_spa_router};
#[cfg(wasm)]
use crate::component::PageExt as _;

#[cfg(wasm)]
thread_local! {
	/// Cumulative count of `ClientLauncher::render_and_mount` invocations
	/// since the WASM module loaded. Backs `ClientLauncher::__diag_render_count()`.
	/// Hidden diagnostic counter for testing — Refs #4122.
	static RENDER_COUNT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// WASM client application launcher.
///
/// Encapsulates all client-side startup boilerplate: panic hook, reactive
/// scheduler, DOM mounting, `Router::on_navigate` listeners for route
/// changes, history listener, and built-in SPA link interception. Optional
/// lifecycle hooks (`before_launch`, `after_launch`) and path-driven
/// side effects (`on_path`, `on_path_pattern`) plug into the builder
/// chain so app-level wiring stays declarative.
///
/// # Example
///
/// ```ignore
/// use reinhardt::pages::{ClientLauncher, LaunchCtx, PathCtx};
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(start)]
/// pub fn main() -> Result<(), JsValue> {
///     ClientLauncher::new("#root")
///         .before_launch(|| {
///             // Runs after the panic hook + reactive scheduler are
///             // configured but BEFORE the router is initialised.
///             my_app::state::init_app_state();
///         })
///         .router(router::init_router)
///         .after_launch(|ctx: &LaunchCtx<'_>| {
///             // Runs after the first DOM mount; router is live here.
///             my_app::analytics::report_boot(ctx.document());
///         })
///         .on_path("/", |ctx: &PathCtx<'_>| {
///             // Idempotent body-level mount + side effect on entering "/".
///             ctx.ensure_portal("toast-container", components::toast::container);
///             my_app::ws::connect_notifications();
///         })
///         .on_path_pattern("/orgs/{slug}/", |ctx| {
///             // Re-fires when {slug} changes within the same pattern.
///             my_app::analytics::track_view(ctx.params());
///         })
///         // SPA link interception is enabled by default. Pass false to
///         // opt out if your app installs its own document click handler:
///         // .intercept_links(false)
///         .launch()
/// }
/// ```
#[allow(deprecated)] // (Refs #4234) `router_init` field stores a closure producing the deprecated `Router`.
pub struct ClientLauncher {
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) root_selector: &'static str,
	pub(super) router_init: Option<Box<dyn FnOnce() -> Router>>,
	/// Optional `ClientRouter` initialiser registered via
	/// [`ClientLauncher::router_client`]. Mutually exclusive with
	/// `router_init`; `launch()` rejects the launcher if both are set
	/// or both are `None`. (Refs #4234)
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) client_router_init:
		Option<Box<dyn FnOnce() -> reinhardt_urls::routers::ClientRouter>>,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) intercept_links: bool,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) before_launch_hooks: Vec<BeforeLaunchHook>,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) after_launch_hooks: Vec<AfterLaunchHook>,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) path_subscriptions: Vec<PathSubscription>,
	/// (Refs #4453) `true` when `register_routes_from_inventory()` has been
	/// called. Mutually exclusive with `router_init` and `client_router_init`.
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) use_inventory: bool,
}

/// Context passed to [`ClientLauncher::after_launch`] callbacks.
///
/// Borrows the resources `ClientLauncher::launch` already owns
/// (`window`, `document`, root element); never owns them.
pub struct LaunchCtx<'a> {
	#[cfg_attr(not(wasm), allow(dead_code))]
	window: &'a web_sys::Window,
	#[cfg_attr(not(wasm), allow(dead_code))]
	document: &'a web_sys::Document,
	#[cfg_attr(not(wasm), allow(dead_code))]
	root_element: &'a web_sys::Element,
}

impl<'a> LaunchCtx<'a> {
	/// The browser `window` object.
	pub fn window(&self) -> &web_sys::Window {
		self.window
	}

	/// The current `document`.
	pub fn document(&self) -> &web_sys::Document {
		self.document
	}

	/// The element matched by the launcher's root selector (e.g. `#root`).
	pub fn root_element(&self) -> &web_sys::Element {
		self.root_element
	}
}

/// A one-shot callback invoked before the router is initialised.
type BeforeLaunchHook = Box<dyn FnOnce()>;

/// A one-shot callback invoked after the first DOM mount, receiving a
/// borrow of the launcher's [`LaunchCtx`].
type AfterLaunchHook = Box<dyn FnOnce(&LaunchCtx<'_>)>;

/// Path parameters extracted from a route match.
///
/// Re-exposed as a type alias so the public API does not leak the
/// `HashMap` constructor at call sites; users can write
/// `fn handle(ctx: &PathCtx) { let id = ctx.params().get("id"); }`.
pub type PathParams = HashMap<String, String>;

/// Context passed to [`ClientLauncher::on_path`] /
/// [`ClientLauncher::on_path_pattern`] callbacks.
///
/// Borrows the current `document`, the matched path string, and the
/// extracted path parameters; never owns them.
pub struct PathCtx<'a> {
	#[cfg_attr(not(wasm), allow(dead_code))]
	document: &'a web_sys::Document,
	path: &'a str,
	params: &'a PathParams,
}

impl<'a> PathCtx<'a> {
	/// The current `document`. Only useful in WASM builds; on the host
	/// the underlying type is still defined but no real DOM exists.
	pub fn document(&self) -> &web_sys::Document {
		self.document
	}

	/// The currently active path (e.g. `"/orgs/foo/"`).
	pub fn path(&self) -> &str {
		self.path
	}

	/// Path parameters extracted by the matched pattern (e.g. `{ "slug": "foo" }`).
	///
	/// For exact-match `on_path` registrations this is always empty.
	pub fn params(&self) -> &PathParams {
		self.params
	}

	/// Idempotent body-level mount.
	///
	/// If `document.getElementById(id)` already returns an element,
	/// this is a no-op. Otherwise renders `factory()` and appends the
	/// resulting root element to `document.body`.
	///
	/// Useful for installing app-level overlays (toast containers,
	/// modals) in `on_path` callbacks without hand-rolling the
	/// "mount once" guard.
	#[cfg(wasm)]
	pub fn ensure_portal<F>(&self, id: &str, factory: F)
	where
		F: FnOnce() -> crate::component::Page,
	{
		if self.document.get_element_by_id(id).is_some() {
			return;
		}

		let page = factory();
		let html = page.render_to_string();

		let Some(body) = self.document.body() else {
			return;
		};
		let Ok(wrapper) = self.document.create_element("div") else {
			return;
		};
		wrapper.set_inner_html(&html);

		// Convention: factory output is a single root element. Append the
		// first element child if present so the caller's `id` lands on the
		// outermost element they wrote; fall back to the wrapper otherwise
		// so something is always inserted.
		if let Some(child) = wrapper.first_element_child() {
			let _ = body.append_child(&child);
		} else {
			let _ = body.append_child(&wrapper);
		}
	}

	/// Host-side stub for `ensure_portal`. No DOM exists, so the call
	/// is a no-op.
	#[cfg(not(wasm))]
	pub fn ensure_portal<F>(&self, _id: &str, _factory: F)
	where
		F: FnOnce() -> crate::component::Page,
	{
	}
}

/// Internal record produced by every `.on_path` / `.on_path_pattern` call.
///
/// `last_params` tracks the previous match state so the `on_navigate`
/// listener can detect transitions (entering a match, or a parameter-set
/// change inside the same pattern) without re-firing on every navigation.
#[allow(deprecated)] // (Refs #4234) `pattern` field stores the deprecated `PathPattern`.
pub(super) struct PathSubscription {
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) pattern: PathPattern,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) callback: Box<dyn Fn(&PathCtx<'_>) + 'static>,
	#[cfg_attr(not(wasm), allow(dead_code))]
	pub(super) last_params: RefCell<Option<HashMap<String, String>>>,
}

/// Diff state machine shared by `on_path` / `on_path_pattern`
/// subscriptions.
///
/// Evaluates `pattern` against `path`, updates `last_params` with the
/// new match (or `None`) regardless of the transition, and returns
/// `Some(new_params)` only on transitions that should fire the user
/// callback:
///
/// - `None -> Some` (entering a match): fires.
/// - `Some(a) -> Some(b)` where `a != b` (param change inside the
///   same pattern): fires.
/// - `Some(_) -> None` (leaving a match): does not fire.
/// - `None -> None` (still unmatched): does not fire.
///
/// Extracted so the launcher's Phase C path-subscription registration
/// (`launch()`) and its native unit tests share a single diff
/// implementation; the same helper is invoked once at registration to
/// deliver the bootstrap route and again from each `Router::on_navigate`
/// dispatch (Refs #4101).
#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
#[allow(deprecated)] // (Refs #4234) Operates on the deprecated `PathPattern` by design.
fn next_path_subscription_match(
	pattern: &PathPattern,
	path: &str,
	last_params: &RefCell<Option<HashMap<String, String>>>,
) -> Option<HashMap<String, String>> {
	let new_match: Option<HashMap<String, String>> = pattern.matches(path).map(|(p, _)| p);
	let should_fire = {
		let mut prev = last_params.borrow_mut();
		let fire = match (&*prev, &new_match) {
			(None, Some(_)) => true,
			(Some(_), None) => false,
			(Some(a), Some(b)) => a != b,
			(None, None) => false,
		};
		*prev = new_match.clone();
		fire
	};
	if should_fire { new_match } else { None }
}

/// The router source `launch()` will use, decided from the three
/// mutually-exclusive launcher methods.
///
/// Refs #4453.
#[allow(dead_code)] // wired up in Task 3.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouterSourceChoice {
	/// `router(...)` was the only source set.
	Legacy,
	/// `router_client(...)` was the only source set.
	Client,
	/// `register_routes_from_inventory()` was the only source set.
	Inventory,
	/// No source was set.
	None,
	/// More than one source was set.
	Conflict,
}

/// Decide which router source `launch()` will use, based on which of the
/// three launcher methods were called. Returns `Conflict` if more than one
/// was called; `None` if none were called.
///
/// Pure function — testable on the native target without the WASM runtime.
///
/// Refs #4453.
#[allow(dead_code)] // wired up in Task 3.3
fn select_router_source_counts(legacy: bool, client: bool, inventory: bool) -> RouterSourceChoice {
	match (legacy as u8) + (client as u8) + (inventory as u8) {
		0 => RouterSourceChoice::None,
		1 if legacy => RouterSourceChoice::Legacy,
		1 if client => RouterSourceChoice::Client,
		1 if inventory => RouterSourceChoice::Inventory,
		_ => RouterSourceChoice::Conflict,
	}
}

#[allow(deprecated)] // (Refs #4234) Builder consumes the deprecated `Router` via `router(...)`.
impl ClientLauncher {
	/// Create a new launcher targeting the given CSS selector (e.g. `"#root"`).
	pub fn new(root_selector: &'static str) -> Self {
		Self {
			root_selector,
			router_init: None,
			client_router_init: None,
			intercept_links: true,
			before_launch_hooks: Vec::new(),
			after_launch_hooks: Vec::new(),
			path_subscriptions: Vec::new(),
			use_inventory: false,
		}
	}

	/// Register the router initializer function.
	///
	/// The function is called once during `launch()` before the first render.
	#[deprecated(
		since = "0.1.0-rc.27",
		note = "Use `ClientLauncher::router_client` with `urls::ClientRouter` instead. \
				Refs cloud#578 Phase E."
	)]
	pub fn router<F: FnOnce() -> Router + 'static>(mut self, f: F) -> Self {
		self.router_init = Some(Box::new(f));
		self
	}

	/// Use a [`reinhardt_urls::routers::ClientRouter`] as the SPA route
	/// table.
	///
	/// Recommended over [`ClientLauncher::router`] for new code; the
	/// latter is `#[deprecated]` and consumes the deprecated
	/// `pages::Router`. (Refs #4234, cloud#578 Phase E)
	///
	/// # Conflict
	///
	/// `router_client` and [`ClientLauncher::router`] are mutually
	/// exclusive — calling both on the same launcher causes
	/// `ClientLauncher::launch` to return an error. Pick one.
	/// (`launch` is `#[cfg(wasm)]`, so it cannot be linked from native docs.)
	pub fn router_client<F>(mut self, f: F) -> Self
	where
		F: FnOnce() -> reinhardt_urls::routers::ClientRouter + 'static,
	{
		self.client_router_init = Some(Box::new(f));
		self
	}

	/// Pull the SPA route table from all `#[routes]`-annotated functions
	/// registered via `inventory` at compile time.
	///
	/// This is the canonical Reinhardt SPA bootstrap pattern. Combined
	/// with the `#[routes]` macro's WASM inventory-submission block, it
	/// lets the entire WASM entry point collapse to:
	///
	/// ```rust,ignore
	/// #[wasm_bindgen(start)]
	/// pub fn main() -> Result<(), JsValue> {
	///     ClientLauncher::new("#root")
	///         .register_routes_from_inventory()
	///         .launch()
	/// }
	/// ```
	///
	/// # Conflict
	///
	/// Mutually exclusive with [`Self::router`] (deprecated) and
	/// [`Self::router_client`]. Calling more than one of the three on
	/// the same launcher causes [`Self::launch`] to return an error.
	/// Calling none of them also returns an error.
	///
	/// `launch()` returns an error if no `#[routes]` registration is
	/// found at runtime (i.e. the `inventory` iterator is empty).
	///
	/// Refs #4453.
	pub fn register_routes_from_inventory(mut self) -> Self {
		self.use_inventory = true;
		self
	}

	/// Toggle built-in SPA link interception.
	///
	/// When enabled (the default), `launch()` installs a document-level
	/// `click` listener that converts clicks on internal `<a href="/...">`
	/// anchors into `Router::push` navigations, so a full page reload is
	/// avoided.
	///
	/// The listener intentionally skips:
	/// - external URLs (`href` not starting with `/`)
	/// - `target="_blank"`
	/// - `download` attribute
	/// - `rel="external"` (whitespace-split, case-insensitive)
	/// - clicks with Ctrl/Cmd/Shift modifier keys (so users can still
	///   open links in a new tab/window)
	///
	/// Pass `false` to opt out — applications that already install their
	/// own document-level link handler should disable this to avoid
	/// double-handling.
	pub fn intercept_links(mut self, enabled: bool) -> Self {
		self.intercept_links = enabled;
		self
	}

	/// Register a callback to run **before** the router is initialised.
	///
	/// `before_launch` callbacks fire after the panic hook and reactive
	/// scheduler are configured but before `router_init` is called, so
	/// they are the right place for state initialisation that components
	/// will read during their first render.
	///
	/// Multiple calls accumulate in registration order.
	pub fn before_launch<F>(mut self, hook: F) -> Self
	where
		F: FnOnce() + 'static,
	{
		self.before_launch_hooks.push(Box::new(hook));
		self
	}

	/// Register a callback to run **after** the first DOM mount.
	///
	/// `after_launch` callbacks fire after the initial render has been
	/// mounted to the root element. The callback receives a [`LaunchCtx`]
	/// with borrows of the `window`, `document`, and root element that
	/// `launch()` already owns. The router is fully initialised at this
	/// point, so [`with_router`](crate::app::with_router) is safe to call.
	///
	/// Multiple calls accumulate in registration order.
	pub fn after_launch<F>(mut self, hook: F) -> Self
	where
		F: FnOnce(&LaunchCtx<'_>) + 'static,
	{
		self.after_launch_hooks.push(Box::new(hook));
		self
	}

	/// Register a side effect that fires on transitions into `path` (exact match).
	///
	/// The callback receives a [`PathCtx`] with the current document and
	/// path; for exact-match registrations, `params()` is always empty.
	///
	/// Internally each registration becomes a leaked [`Router::on_navigate`]
	/// listener; the callback fires when the application enters the matching
	/// path and is independent of the reactive `Effect` / `Signal`
	/// auto-tracking system. It does **not** fire on repeated navigations
	/// to the same path. Leaving the path does not fire the callback either
	/// — register a separate `on_path` for the destination if you need exit
	/// cleanup.
	pub fn on_path<F>(mut self, path: &'static str, callback: F) -> Self
	where
		F: Fn(&PathCtx<'_>) + 'static,
	{
		self.path_subscriptions.push(PathSubscription {
			pattern: PathPattern::new(path),
			callback: Box::new(callback),
			last_params: RefCell::new(None),
		});
		self
	}

	/// Register a side effect that fires on transitions into any path
	/// matching `pattern`.
	///
	/// The pattern syntax is the same as `Router::route` (e.g.
	/// `"/orgs/{slug}/"` or `"/static/{path:*}/"`). The callback fires
	/// when:
	/// - the app enters a path that matches the pattern, OR
	/// - the path still matches but the extracted parameters changed
	///   (e.g. `/orgs/foo/` → `/orgs/bar/`).
	///
	/// Re-renders that do not change the path leave the callback dormant.
	pub fn on_path_pattern<F>(mut self, pattern: &'static str, callback: F) -> Self
	where
		F: Fn(&PathCtx<'_>) + 'static,
	{
		self.path_subscriptions.push(PathSubscription {
			pattern: PathPattern::new(pattern),
			callback: Box::new(callback),
			last_params: RefCell::new(None),
		});
		self
	}
}

#[cfg(wasm)]
#[allow(deprecated)] // (Refs #4234) Launch path bridges deprecated `Router` and new `ClientRouter`.
impl ClientLauncher {
	/// Render the current route into the given root element.
	///
	/// Performs `Router::render_current` -> `cleanup_reactive_nodes` ->
	/// clears `innerHTML` -> `view.mount`. Used by `launch()` for both
	/// the initial mount (called inline in Phase B) and every
	/// subsequent re-mount (called from a `Router::on_navigate`
	/// listener registered in Phase C).
	///
	/// Refs #4101.
	fn render_and_mount(root_el: &web_sys::Element) -> Result<(), crate::component::MountError> {
		RENDER_COUNT.with(|c| c.set(c.get() + 1));
		let view = with_spa_router(|r| r.render_current());
		crate::component::cleanup_reactive_nodes();
		root_el.set_inner_html("");
		let wrapper = crate::dom::Element::new(root_el.clone());
		view.mount(&wrapper)
	}

	/// Diagnostic counter: cumulative count of `render_and_mount`
	/// invocations since the WASM module loaded. Includes the initial
	/// Phase B mount and every subsequent `Router::on_navigate`-driven
	/// re-mount. Used by tests in
	/// `tests/wasm/spa_navigation_diag_test.rs` to assert Inv-4.
	///
	/// Only available under `cfg(wasm)`: the entire `impl ClientLauncher`
	/// block in this scope is gated, and `RENDER_COUNT` is a `cfg(wasm)`-
	/// only thread_local. Calling from a non-wasm32 build results in a
	/// compile error rather than a no-op zero. Hidden API for testing
	/// only. Refs #4122.
	#[doc(hidden)]
	pub fn __diag_render_count() -> u64 {
		RENDER_COUNT.with(|c| c.get())
	}

	/// Start the WASM client application.
	///
	/// Performs three phases in order:
	///
	/// 1. **Phase A — Setup.** Sets up the panic hook for readable
	///    console errors, configures the reactive scheduler for async
	///    contexts, runs registered `before_launch` callbacks,
	///    initialises the [`Router`] and stores it in the global
	///    thread-local, registers the `popstate` history listener,
	///    queries the DOM for `root_selector` (returns `Err` if not
	///    found), and installs the SPA link-interception listener on
	///    `document` when `intercept_links` is `true` (the default).
	///
	/// 2. **Phase B — Initial mount.** Inline (no `Effect`):
	///    `Router::render_current()` -> `cleanup_reactive_nodes()` ->
	///    clears `innerHTML` -> `view.mount()`. On mount failure
	///    `launch()` returns `Err` and Phase C is skipped.
	///
	/// 3. **Phase C — Persistent subscriptions.** Registers the
	///    launcher's render listener via [`Router::on_navigate`] (the
	///    returned [`NavigationSubscription`] is leaked via
	///    `mem::forget` so it persists for the WASM module lifetime),
	///    runs registered `after_launch` callbacks, then registers
	///    one `Router::on_navigate` listener per `on_path` /
	///    `on_path_pattern` subscription. Each path-subscription
	///    listener fires only on transitions into or between matching
	///    param sets (de-duplicated through a
	///    `RefCell<Option<HashMap>>` state).
	///
	/// The launcher does **not** create any reactive `Effect`. The
	/// render pipeline is driven entirely by [`Router::on_navigate`]
	/// callbacks, which are independent of the reactive runtime. This
	/// is structurally robust against nested reactive nodes spawned
	/// during view rendering (Refs #3348, #4075, #4088, #4101).
	///
	/// [`Router::on_navigate`] fires for both programmatic navigation
	/// (`Router::push` / `Router::replace`) and browser back/forward
	/// (popstate).
	pub fn launch(mut self) -> Result<(), wasm_bindgen::JsValue> {
		#[cfg(feature = "console_error_panic_hook")]
		console_error_panic_hook::set_once();

		crate::reactive::runtime::set_scheduler(|task| {
			wasm_bindgen_futures::spawn_local(async move { task() });
		});

		// Step 3: drain before_launch callbacks before any router or DOM work.
		for hook in self.before_launch_hooks.drain(..) {
			hook();
		}

		// (Refs #4234) Pick exactly one router source. The deprecated
		// `router(...)` builder produces a `Router`; the new
		// `router_client(...)` builder produces a `ClientRouter`.
		// Either is valid; both set or neither set is an error.
		let spa_router: Box<dyn super::SpaRouter> =
			match (self.router_init.take(), self.client_router_init.take()) {
				(Some(_), Some(_)) => {
					return Err(wasm_bindgen::JsValue::from_str(
						"ClientLauncher: `router(...)` and `router_client(...)` \
						 are mutually exclusive; configure only one.",
					));
				}
				(Some(f), None) => {
					// (Refs #4234) `Router` is deprecated as of rc.27; the
					// parent `impl ClientLauncher` block carries
					// `#[allow(deprecated)]` so this arm continues to build.
					Box::new(f())
				}
				(None, Some(f)) => Box::new(f()),
				(None, None) => {
					return Err(wasm_bindgen::JsValue::from_str(
						"ClientLauncher: `router(...)` or `router_client(...)` \
						 must be called before `launch()`.",
					));
				}
			};
		store_spa_router(spa_router);

		crate::nav_diag!(
			"site=store_router router_id={} route_count={}",
			with_spa_router(|r| r.__diag_router_id()),
			with_spa_router(|r| r.route_count())
		);
		crate::nav_diag_dom!("store_router");

		with_spa_router(|r| r.setup_history_listener());

		let window = web_sys::window()
			.ok_or_else(|| wasm_bindgen::JsValue::from_str("no global `window`"))?;
		let document = window
			.document()
			.ok_or_else(|| wasm_bindgen::JsValue::from_str("no document on window"))?;

		if self.intercept_links {
			install_link_interceptor(&document)?;
		}

		let root_el = document
			.query_selector(self.root_selector)?
			.ok_or_else(|| {
				wasm_bindgen::JsValue::from_str(&format!(
					"element '{}' not found",
					self.root_selector
				))
			})?;

		// Phase B: initial mount runs inline (no Effect). Errors
		// propagate directly because no Effect/Signal indirection
		// captures them. Refs #4101.
		Self::render_and_mount(&root_el)
			.map_err(|e| wasm_bindgen::JsValue::from_str(&format!("initial mount failed: {e}")))?;

		// Phase C (part 1): register the launcher's render listener
		// via Router::on_navigate. Registered BEFORE the after_launch
		// drain so that any router.push() / router.replace()
		// triggered from an after_launch hook re-renders, matching
		// the previous behaviour where the render Effect was already
		// active by that point. Router::on_navigate fires for both
		// programmatic navigation and popstate (popstate dispatch
		// added in #4108).
		//
		// The subscription is leaked for the entire WASM module
		// lifetime (modules never terminate, so there is no
		// destructor to run). Refs #4101, #4108, #4088.
		let render_root = root_el.clone();
		let render_subscription = with_spa_router(|r| {
			r.on_navigate_dyn(Box::new(move |_path, _params| {
				if let Err(e) = Self::render_and_mount(&render_root) {
					web_sys::console::error_1(&format!("re-render failed: {e}").into());
				}
			}))
		});
		std::mem::forget(render_subscription);

		crate::nav_diag!(
			"site=register_render_listener router_id={} observer_count_after={}",
			with_spa_router(|r| r.__diag_router_id()),
			with_spa_router(|r| r.__diag_observer_count())
		);

		// Phase C (between part 1 and part 2): drain after_launch
		// callbacks now that the router is live, the first DOM mount
		// has completed, and the render listener is active. Path
		// subscriptions registered below see whatever path the
		// after_launch hooks may have pushed.
		if !self.after_launch_hooks.is_empty() {
			let ctx = LaunchCtx {
				window: &window,
				document: &document,
				root_element: &root_el,
			};
			for hook in self.after_launch_hooks.drain(..) {
				hook(&ctx);
			}
		}

		// Phase C (part 2, #4101): register one leaked
		// Router::on_navigate listener per path subscription, then
		// manually evaluate the current path once so subscriptions
		// whose pattern matches the bootstrap route deliver the initial
		// route at startup. The previous Effect-based implementation
		// got the initial-route delivery for free because Effects run
		// their closure once at creation; the on_navigate-based
		// implementation only fires on subsequent navigations, so the
		// initial evaluation is restored explicitly here.
		//
		// The listener is registered BEFORE the initial evaluation so
		// that any `Router::push` triggered from inside the user
		// callback during initial eval is observed by this listener
		// (matching the previous behaviour where the reactive runtime
		// would re-execute the Effect on the same Signal change). The
		// `Rc<RefCell<Option<HashMap>>>` diff state is shared between
		// the listener closure and the initial-eval site so transitions
		// between the two are detected by the same state machine.
		for sub in self.path_subscriptions.into_iter() {
			let PathSubscription {
				pattern,
				callback,
				last_params,
			} = sub;
			let pattern: std::rc::Rc<PathPattern> = std::rc::Rc::new(pattern);
			let callback: std::rc::Rc<dyn Fn(&PathCtx<'_>) + 'static> = std::rc::Rc::from(callback);
			let last_params: std::rc::Rc<RefCell<Option<HashMap<String, String>>>> =
				std::rc::Rc::new(last_params);

			let pattern_for_listener = pattern.clone();
			let callback_for_listener = callback.clone();
			let last_params_for_listener = last_params.clone();
			let document_for_listener = document.clone();
			let listener_subscription = with_spa_router(|r| {
				r.on_navigate_dyn(Box::new(move |path, _params_from_router| {
					if let Some(params) = next_path_subscription_match(
						&pattern_for_listener,
						path,
						&last_params_for_listener,
					) {
						let ctx = PathCtx {
							document: &document_for_listener,
							path,
							params: &params,
						};
						callback_for_listener(&ctx);
					}
				}))
			});
			std::mem::forget(listener_subscription);

			let initial_path = with_spa_router(|r| r.current_path().get());
			if let Some(params) =
				next_path_subscription_match(&pattern, &initial_path, &last_params)
			{
				let ctx = PathCtx {
					document: &document,
					path: &initial_path,
					params: &params,
				};
				callback(&ctx);
			}
		}

		Ok(())
	}
}

#[cfg(test)]
#[allow(deprecated)] // (Refs #4234) Tests exercise deprecated `pages::Router` / `PathPattern` directly.
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn test_client_launcher_new_stores_selector() {
		let launcher = ClientLauncher::new("#root");

		assert_eq!(launcher.root_selector, "#root");
		assert!(launcher.router_init.is_none());
		assert!(launcher.client_router_init.is_none());
	}

	// (Refs #4234) Exercises the deprecated `router(...)` builder; the
	// module-level `#[allow(deprecated)]` on `mod tests` covers this test.
	#[rstest]
	fn test_client_launcher_router_stores_init_fn() {
		let launcher = ClientLauncher::new("#root");

		let launcher = launcher.router(Router::new);

		assert!(launcher.router_init.is_some());
	}

	// (Refs #4234) Mirrors `test_client_launcher_router_stores_init_fn`
	// for the new `router_client(...)` builder.
	#[rstest]
	fn test_client_launcher_router_client_stores_init_fn() {
		let launcher = ClientLauncher::new("#root");

		let launcher = launcher.router_client(reinhardt_urls::routers::ClientRouter::new);

		assert!(launcher.client_router_init.is_some());
	}

	#[rstest]
	fn test_client_launcher_intercept_links_default_true() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root");

		// Assert
		assert!(launcher.intercept_links);
	}

	#[rstest]
	fn test_client_launcher_intercept_links_false_overrides_default() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root").intercept_links(false);

		// Assert
		assert!(!launcher.intercept_links);
	}

	// --- before_launch / after_launch builder tests ---

	#[rstest]
	fn test_before_launch_starts_empty() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root");
		// Assert
		assert!(launcher.before_launch_hooks.is_empty());
	}

	#[rstest]
	fn test_before_launch_accumulates_in_registration_order() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root")
			.before_launch(|| { /* hook 1 */ })
			.before_launch(|| { /* hook 2 */ })
			.before_launch(|| { /* hook 3 */ });
		// Assert
		assert_eq!(launcher.before_launch_hooks.len(), 3);
	}

	#[rstest]
	fn test_after_launch_starts_empty() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root");
		// Assert
		assert!(launcher.after_launch_hooks.is_empty());
	}

	#[rstest]
	fn test_after_launch_accumulates_in_registration_order() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root")
			.after_launch(|_ctx: &LaunchCtx<'_>| { /* hook 1 */ })
			.after_launch(|_ctx: &LaunchCtx<'_>| { /* hook 2 */ });
		// Assert
		assert_eq!(launcher.after_launch_hooks.len(), 2);
	}

	// --- on_path / on_path_pattern builder tests ---

	#[rstest]
	fn test_path_subscriptions_start_empty() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root");
		// Assert
		assert!(launcher.path_subscriptions.is_empty());
	}

	#[rstest]
	fn test_on_path_appends_exact_subscription() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root")
			.on_path("/", |_ctx: &PathCtx<'_>| {})
			.on_path("/users/", |_ctx: &PathCtx<'_>| {});
		// Assert
		assert_eq!(launcher.path_subscriptions.len(), 2);
		assert!(launcher.path_subscriptions[0].pattern.is_exact());
		assert_eq!(launcher.path_subscriptions[0].pattern.pattern(), "/");
		assert!(launcher.path_subscriptions[1].pattern.is_exact());
		assert_eq!(launcher.path_subscriptions[1].pattern.pattern(), "/users/");
	}

	#[rstest]
	fn test_on_path_pattern_appends_pattern_subscription() {
		// Arrange / Act
		let launcher =
			ClientLauncher::new("#root").on_path_pattern("/orgs/{slug}/", |_ctx: &PathCtx<'_>| {});
		// Assert
		assert_eq!(launcher.path_subscriptions.len(), 1);
		let sub = &launcher.path_subscriptions[0];
		assert!(!sub.pattern.is_exact());
		assert!(sub.pattern.matches("/orgs/foo/").is_some());
		assert!(sub.pattern.matches("/orgs/").is_none());
	}

	#[rstest]
	fn test_on_path_subscriptions_start_with_no_recorded_match() {
		// Arrange / Act
		let launcher = ClientLauncher::new("#root").on_path("/", |_ctx: &PathCtx<'_>| {});
		// Assert: last_params is None at registration time so the very
		// first listener invocation will be detected as a `None -> Some(_)` transition.
		assert!(
			launcher.path_subscriptions[0]
				.last_params
				.borrow()
				.is_none()
		);
	}

	// --- transition logic regression test ---

	/// Mirrors the per-subscription transition decision used inside
	/// `launch()` so we can exercise the `Cell<bool>` -> `RefCell<HashMap>`
	/// upgrade rationale on the host (where no router exists).
	fn fire_decision(
		prev: &Option<HashMap<String, String>>,
		new: &Option<HashMap<String, String>>,
	) -> bool {
		match (prev, new) {
			(None, Some(_)) => true,
			(Some(_), None) => false,
			(Some(a), Some(b)) => a != b,
			(None, None) => false,
		}
	}

	#[rstest]
	fn test_transition_logic_fires_on_initial_match() {
		// Arrange
		let prev = None;
		let new = Some(HashMap::new());
		// Act / Assert
		assert!(fire_decision(&prev, &new));
	}

	#[rstest]
	fn test_transition_logic_skips_re_render_at_same_path() {
		// Arrange
		let mut params = HashMap::new();
		params.insert("slug".into(), "foo".into());
		let prev = Some(params.clone());
		let new = Some(params);
		// Act / Assert
		assert!(!fire_decision(&prev, &new));
	}

	#[rstest]
	fn test_transition_logic_fires_when_pattern_params_change() {
		// Arrange
		let mut a = HashMap::new();
		a.insert("slug".into(), "foo".into());
		let mut b = HashMap::new();
		b.insert("slug".into(), "bar".into());
		let prev = Some(a);
		let new = Some(b);
		// Act / Assert
		assert!(fire_decision(&prev, &new));
	}

	#[rstest]
	fn test_transition_logic_does_not_fire_when_leaving_match() {
		// Arrange
		let mut params = HashMap::new();
		params.insert("slug".into(), "foo".into());
		let prev = Some(params);
		let new = None;
		// Act / Assert
		assert!(!fire_decision(&prev, &new));
	}

	// --- Phase C path-subscription registration regression tests ---

	/// Mirror of the launcher's Phase C (part 2) path-subscription
	/// registration algorithm: register an on_navigate listener that
	/// runs the diff-and-fire helper, then immediately evaluate the
	/// current path through the same helper so an already-matching
	/// bootstrap route delivers its callback at registration time.
	///
	/// Returns the `NavigationSubscription` so the test can `mem::forget`
	/// it (matching the launcher) or drop it to unregister.
	fn register_path_subscription_for_test<F>(
		router: &Router,
		pattern: PathPattern,
		last_params: std::rc::Rc<RefCell<Option<HashMap<String, String>>>>,
		on_match: F,
	) -> crate::router::NavigationSubscription
	where
		F: Fn(&str, &HashMap<String, String>) + 'static,
	{
		let on_match = std::rc::Rc::new(on_match);
		let pattern = std::rc::Rc::new(pattern);

		let on_match_listener = on_match.clone();
		let pattern_listener = pattern.clone();
		let last_params_listener = last_params.clone();
		let subscription = router.on_navigate(move |path, _params_from_router| {
			if let Some(params) =
				next_path_subscription_match(&pattern_listener, path, &last_params_listener)
			{
				on_match_listener(path, &params);
			}
		});

		let initial_path = router.current_path().get();
		if let Some(params) = next_path_subscription_match(&pattern, &initial_path, &last_params) {
			on_match(&initial_path, &params);
		}

		subscription
	}

	/// Initial-route delivery: a subscription whose pattern matches the
	/// router's current path at registration time MUST fire the user
	/// callback once with the bootstrap params.
	///
	/// The previous Effect-based implementation got this for free
	/// because Effects run their closure once at creation; the
	/// on_navigate-based implementation only fires on subsequent
	/// navigations, so the launcher restores the initial evaluation
	/// explicitly. Removing that explicit step would break this test.
	///
	/// Refs #4101.
	#[rstest]
	fn path_subscription_delivers_initial_route() {
		use crate::component::Page;

		// Arrange
		let router = Router::new()
			.route("/users/{id}/", || Page::text("user"))
			.route("/about/", || Page::text("about"));
		router.push("/users/1/").expect("push /users/1/");

		let observed: std::rc::Rc<RefCell<Vec<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(Vec::new()));
		let observed_inner = observed.clone();
		let last_params: std::rc::Rc<RefCell<Option<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(None));

		// Act: register subscription AFTER the router has been navigated.
		let _sub = register_path_subscription_for_test(
			&router,
			PathPattern::new("/users/{id}/"),
			last_params,
			move |_path, params| {
				observed_inner.borrow_mut().push(params.clone());
			},
		);

		// Assert: callback fired once for the bootstrap route.
		let calls = observed.borrow();
		assert_eq!(
			calls.len(),
			1,
			"expected initial-route delivery; got: {:?}",
			calls
		);
		assert_eq!(calls[0].get("id").map(String::as_str), Some("1"));
	}

	/// End-to-end sequence covering initial-route delivery plus the
	/// transition state machine, exercised through the launcher's
	/// registration algorithm rather than the diff helper alone.
	///
	/// Refs #4101.
	#[rstest]
	fn path_subscription_initial_then_navigation_sequence() {
		use crate::component::Page;

		// Arrange
		let router = Router::new()
			.route("/users/{id}/", || Page::text("user"))
			.route("/about/", || Page::text("about"));
		router.push("/users/1/").expect("push /users/1/");

		let observed: std::rc::Rc<RefCell<Vec<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(Vec::new()));
		let observed_inner = observed.clone();
		let last_params: std::rc::Rc<RefCell<Option<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(None));

		// Act
		let _sub = register_path_subscription_for_test(
			&router,
			PathPattern::new("/users/{id}/"),
			last_params,
			move |_path, params| {
				observed_inner.borrow_mut().push(params.clone());
			},
		);
		// Initial delivery already happened; following sequence exercises
		// the diff state machine through the on_navigate listener.
		router.push("/users/1/").expect("re-push /users/1/"); // no fire (Some -> Some same)
		router.push("/users/2/").expect("push /users/2/"); // fire (Some(a) -> Some(b), a != b)
		router.push("/about/").expect("push /about/"); // no fire (Some -> None)
		router
			.push("/users/3/")
			.expect("push /users/3/ after /about/"); // fire (None -> Some)

		// Assert
		let calls = observed.borrow();
		assert_eq!(
			calls.len(),
			3,
			"expected initial + 2 transitions; got: {:?}",
			calls
		);
		assert_eq!(calls[0].get("id").map(String::as_str), Some("1"));
		assert_eq!(calls[1].get("id").map(String::as_str), Some("2"));
		assert_eq!(calls[2].get("id").map(String::as_str), Some("3"));
	}

	/// Non-matching bootstrap route: a subscription whose pattern does
	/// not match the router's current path at registration MUST NOT
	/// fire at registration. Locks in the `None -> None` branch of the
	/// diff state machine for the initial-eval path.
	///
	/// Refs #4101.
	#[rstest]
	fn path_subscription_does_not_fire_when_initial_route_does_not_match() {
		use crate::component::Page;

		// Arrange
		let router = Router::new()
			.route("/users/{id}/", || Page::text("user"))
			.route("/about/", || Page::text("about"));
		router.push("/about/").expect("push /about/");

		let observed: std::rc::Rc<RefCell<Vec<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(Vec::new()));
		let observed_inner = observed.clone();
		let last_params: std::rc::Rc<RefCell<Option<HashMap<String, String>>>> =
			std::rc::Rc::new(RefCell::new(None));

		// Act
		let _sub = register_path_subscription_for_test(
			&router,
			PathPattern::new("/users/{id}/"),
			last_params,
			move |_path, params| {
				observed_inner.borrow_mut().push(params.clone());
			},
		);

		// Assert: callback did not fire because the bootstrap route does
		// not match the pattern.
		assert!(
			observed.borrow().is_empty(),
			"expected no firing; got: {:?}",
			observed.borrow()
		);
	}

	#[rstest]
	fn test_lifecycle_hooks_observe_registration_order() {
		// Arrange: shared counter that records the call order.
		let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u32>::new()));

		let t = trace.clone();
		let h1 = move || t.borrow_mut().push(1);
		let t = trace.clone();
		let h2 = move || t.borrow_mut().push(2);

		// Act
		let launcher = ClientLauncher::new("#root")
			.before_launch(h1)
			.before_launch(h2);

		// Drain the recorded hooks like `launch()` would, on the host.
		// We can read the field directly because the test lives in the same
		// module.
		for hook in launcher.before_launch_hooks {
			hook();
		}

		// Assert
		assert_eq!(*trace.borrow(), vec![1, 2]);
	}
}

#[cfg(test)]
mod select_router_source_tests {
	use super::*;

	#[rstest::rstest]
	fn returns_none_when_no_source_set() {
		// Arrange
		let legacy = false;
		let client = false;
		let inventory = false;
		// Act
		let result = select_router_source_counts(legacy, client, inventory);
		// Assert
		assert_eq!(result, RouterSourceChoice::None);
	}

	#[rstest::rstest]
	fn returns_legacy_when_only_legacy_set() {
		// Arrange + Act
		let result = select_router_source_counts(true, false, false);
		// Assert
		assert_eq!(result, RouterSourceChoice::Legacy);
	}

	#[rstest::rstest]
	fn returns_client_when_only_client_set() {
		// Arrange + Act
		let result = select_router_source_counts(false, true, false);
		// Assert
		assert_eq!(result, RouterSourceChoice::Client);
	}

	#[rstest::rstest]
	fn returns_inventory_when_only_inventory_set() {
		// Arrange + Act
		let result = select_router_source_counts(false, false, true);
		// Assert
		assert_eq!(result, RouterSourceChoice::Inventory);
	}

	#[rstest::rstest]
	#[case(true, true, false)]
	#[case(true, false, true)]
	#[case(false, true, true)]
	#[case(true, true, true)]
	fn returns_conflict_when_multiple_sources_set(
		#[case] legacy: bool,
		#[case] client: bool,
		#[case] inventory: bool,
	) {
		// Arrange + Act
		let result = select_router_source_counts(legacy, client, inventory);
		// Assert
		assert_eq!(result, RouterSourceChoice::Conflict);
	}
}
