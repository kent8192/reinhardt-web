//! SSR Renderer for Component-based server-side rendering.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::Duration;

use super::markers::{HydrationMarker, HydrationStrategy};
use super::resource_context::{
	SsrResourceContext, enter_boundary, resolve_boundary_resources, resolve_external_resources,
	resolve_pending_resources, scope_context,
};
use super::state::SsrState;
use super::stream::{SsrChunk, SsrStream};
use crate::auth::AuthData;
use crate::component::{Component, Head, IntoPage, Page};
use crate::reactive::hooks::id::{
	id_counter_snapshot, reset_id_counter, restore_id_counter, scope_id_counter,
	scope_id_counter_with,
};
use futures_util::StreamExt;
use futures_util::future::{FutureExt, LocalBoxFuture};
use futures_util::stream::{self, FuturesUnordered};
use reinhardt_core::reactive::ReactiveScope;
use reinhardt_core::types::page::{BOOLEAN_ATTRS, SuspenseNode, is_boolean_attr_truthy};

/// Options for SSR rendering.
#[derive(Debug, Clone)]
pub struct SsrOptions {
	/// Whether to include hydration markers.
	pub include_hydration_markers: bool,
	/// Whether to minify the output.
	pub minify: bool,
	/// Whether to include SSR state script.
	pub include_state_script: bool,
	/// Language attribute for HTML element.
	pub lang: String,
	/// CSRF token to embed.
	pub csrf_token: Option<String>,
	/// Authentication data to embed.
	pub auth_data: Option<AuthData>,
	/// Enable partial hydration (Island Architecture, Phase 2-B).
	///
	/// When enabled, only components marked as islands are hydrated on the client.
	/// Static content is preserved without hydration, improving performance.
	pub enable_partial_hydration: bool,
	/// Default hydration strategy for components (Phase 2-B).
	///
	/// Determines how unmarked components should be hydrated.
	/// - `Full`: Traditional full hydration (default)
	/// - `Island`: Mark as interactive islands
	/// - `Static`: Mark as static content (no hydration)
	pub default_hydration_strategy: HydrationStrategy,
	/// Maximum time to wait for server resource resolution.
	pub resource_timeout: Duration,
	/// Enables streaming Suspense replacement chunks.
	pub suspense_streaming: bool,
	/// Optional nonce for inline streaming scripts.
	pub script_nonce: Option<String>,
}

impl Default for SsrOptions {
	fn default() -> Self {
		Self {
			include_hydration_markers: true,
			minify: false,
			include_state_script: true,
			lang: "en".to_string(),
			csrf_token: None,
			auth_data: None,
			enable_partial_hydration: false,
			default_hydration_strategy: HydrationStrategy::Full,
			resource_timeout: Duration::from_secs(2),
			suspense_streaming: true,
			script_nonce: None,
		}
	}
}

impl SsrOptions {
	/// Creates new default options.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the language.
	pub fn lang(mut self, lang: impl Into<String>) -> Self {
		self.lang = lang.into();
		self
	}

	/// Sets the reactive i18n context for SSR and hydration.
	#[cfg(feature = "i18n")]
	pub fn i18n_context(mut self, context: crate::i18n::I18nContext) -> SsrRenderConfig {
		self.lang = context.locale_untracked();
		SsrRenderConfig {
			options: self,
			i18n_context: Some(context),
		}
	}

	/// Disables hydration markers.
	pub fn no_hydration(mut self) -> Self {
		self.include_hydration_markers = false;
		self
	}

	/// Enables minification.
	pub fn minify(mut self) -> Self {
		self.minify = true;
		self
	}

	/// Sets the CSRF token.
	pub fn csrf(mut self, token: impl Into<String>) -> Self {
		self.csrf_token = Some(token.into());
		self
	}

	/// Sets the authentication data.
	pub fn auth(mut self, auth_data: AuthData) -> Self {
		self.auth_data = Some(auth_data);
		self
	}

	/// Enables partial hydration (Island Architecture, Phase 2-B).
	///
	/// When enabled, only components marked as islands will be hydrated on the client.
	/// Static content is preserved without hydration, improving performance.
	///
	/// # Example
	///
	/// ```ignore
	/// let options = SsrOptions::new()
	///     .partial_hydration(true)
	///     .default_strategy(HydrationStrategy::Static);
	/// ```
	pub fn partial_hydration(mut self, enable: bool) -> Self {
		self.enable_partial_hydration = enable;
		self
	}

	/// Sets the default hydration strategy (Phase 2-B).
	///
	/// Determines how unmarked components should be hydrated:
	/// - `Full`: Traditional full hydration (default)
	/// - `Island`: Mark as interactive islands
	/// - `Static`: Mark as static content (no hydration)
	///
	/// # Example
	///
	/// ```ignore
	/// let options = SsrOptions::new()
	///     .default_strategy(HydrationStrategy::Island);
	/// ```
	pub fn default_strategy(mut self, strategy: HydrationStrategy) -> Self {
		self.default_hydration_strategy = strategy;
		self
	}

	/// Enables island-only rendering (convenience method, Phase 2-B).
	///
	/// Shortcut for enabling partial hydration with island strategy.
	/// Equivalent to:
	/// ```ignore
	/// options.partial_hydration(true).default_strategy(HydrationStrategy::Island)
	/// ```
	pub fn islands_only(mut self) -> Self {
		self.enable_partial_hydration = true;
		self.default_hydration_strategy = HydrationStrategy::Island;
		self
	}

	/// Sets the server resource timeout.
	pub fn resource_timeout(mut self, timeout: Duration) -> Self {
		self.resource_timeout = timeout;
		self
	}

	/// Enables or disables Suspense streaming.
	pub fn suspense_streaming(mut self, enabled: bool) -> Self {
		self.suspense_streaming = enabled;
		self
	}

	/// Sets the nonce used by inline Suspense replacement scripts.
	pub fn script_nonce(mut self, nonce: impl Into<String>) -> Self {
		self.script_nonce = Some(nonce.into());
		self
	}
}

/// Builder-owned SSR renderer configuration.
#[derive(Debug, Clone)]
pub struct SsrRenderConfig {
	options: SsrOptions,
	#[cfg(feature = "i18n")]
	i18n_context: Option<crate::i18n::I18nContext>,
}

/// Converts public renderer option builders into the renderer's internal config.
pub trait IntoSsrRendererConfig {
	/// Convert into a renderer configuration.
	fn into_ssr_renderer_config(self) -> SsrRenderConfig;
}

impl IntoSsrRendererConfig for SsrOptions {
	fn into_ssr_renderer_config(self) -> SsrRenderConfig {
		SsrRenderConfig {
			options: self,
			#[cfg(feature = "i18n")]
			i18n_context: None,
		}
	}
}

impl IntoSsrRendererConfig for SsrRenderConfig {
	fn into_ssr_renderer_config(self) -> SsrRenderConfig {
		self
	}
}

/// The main SSR renderer.
pub struct SsrRenderer {
	options: SsrOptions,
	#[cfg(feature = "i18n")]
	i18n_context: Option<crate::i18n::I18nContext>,
	state: SsrState,
	rendered_head: Option<Head>,
	hydration_marker_counter: u64,
	suspense_boundary_counter: u64,
	marker_resource_context: Rc<RefCell<SsrResourceContext>>,
	marker_id_counter: Rc<Cell<usize>>,
}

impl Clone for SsrRenderer {
	fn clone(&self) -> Self {
		Self {
			options: self.options.clone(),
			#[cfg(feature = "i18n")]
			i18n_context: self.i18n_context.clone(),
			state: self.state.clone(),
			rendered_head: self.rendered_head.clone(),
			hydration_marker_counter: self.hydration_marker_counter,
			suspense_boundary_counter: self.suspense_boundary_counter,
			marker_resource_context: Rc::new(RefCell::new(SsrResourceContext::new(
				self.options.resource_timeout,
			))),
			marker_id_counter: Rc::new(Cell::new(0)),
		}
	}
}

#[derive(Clone, Copy)]
enum AsyncRenderMode {
	Discovery,
	Buffered,
}

struct PendingSuspenseBoundary {
	boundary_id: String,
	node: SuspenseNode,
	boundary_start: DeterministicRenderSnapshot,
}

#[derive(Clone, Copy)]
struct DeterministicRenderSnapshot {
	resource_call_order_index: Option<usize>,
	suspense_boundary_counter: u64,
	id_counter: usize,
}

type SuspenseBoundaryResult = (PendingSuspenseBoundary, bool);
type SuspenseBoundaryFuture = LocalBoxFuture<'static, Vec<SuspenseBoundaryResult>>;

struct SuspenseStreamRuntime {
	renderer: SsrRenderer,
	reactive_scope: ReactiveScope,
	context: Rc<RefCell<SsrResourceContext>>,
	id_counter: Rc<Cell<usize>>,
	boundaries: FuturesUnordered<SuspenseBoundaryFuture>,
	ready_boundaries: VecDeque<SuspenseBoundaryResult>,
}

enum SuspenseStreamState {
	Shell {
		shell: String,
		runtime: SuspenseStreamRuntime,
	},
	Boundaries(SuspenseStreamRuntime),
	Done,
}

fn suspense_boundary_futures(
	context: &Rc<RefCell<SsrResourceContext>>,
	boundaries: Vec<PendingSuspenseBoundary>,
	#[cfg(feature = "i18n")] i18n_context: Option<crate::i18n::I18nContext>,
) -> FuturesUnordered<SuspenseBoundaryFuture> {
	suspense_boundary_groups(context, boundaries)
		.into_iter()
		.map(|boundaries| {
			let context = Rc::clone(context);
			#[cfg(feature = "i18n")]
			let i18n_context = i18n_context.clone();
			let future = async move {
				let mut results = Vec::new();

				for boundary in boundaries {
					let boundary_resolved =
						resolve_boundary_resources(&context, &boundary.boundary_id).await;
					results.push((boundary, boundary_resolved));
				}

				results
			};
			#[cfg(feature = "i18n")]
			let future = with_i18n_context_future(i18n_context, future);
			future.boxed_local()
		})
		.collect()
}

fn suspense_boundary_groups(
	context: &Rc<RefCell<SsrResourceContext>>,
	boundaries: Vec<PendingSuspenseBoundary>,
) -> Vec<Vec<PendingSuspenseBoundary>> {
	let pending_ids_by_boundary = {
		let context = context.borrow();
		boundaries
			.iter()
			.map(|boundary| {
				context
					.pending_ids_for_boundary(&boundary.boundary_id)
					.into_iter()
					.collect::<BTreeSet<_>>()
			})
			.collect::<Vec<_>>()
	};

	let mut groups: Vec<(BTreeSet<String>, Vec<PendingSuspenseBoundary>)> = Vec::new();
	for (boundary, pending_ids) in boundaries.into_iter().zip(pending_ids_by_boundary) {
		let mut group_pending_ids = pending_ids;
		let mut group_boundaries = vec![boundary];
		let mut index = 0;

		while index < groups.len() {
			if !group_pending_ids.is_empty() && !group_pending_ids.is_disjoint(&groups[index].0) {
				let (pending_ids, mut boundaries) = groups.remove(index);
				group_pending_ids.extend(pending_ids);
				group_boundaries.append(&mut boundaries);
			} else {
				index += 1;
			}
		}

		groups.push((group_pending_ids, group_boundaries));
	}

	groups
		.into_iter()
		.map(|(_, boundaries)| boundaries)
		.collect()
}

#[cfg(feature = "i18n")]
async fn with_i18n_context_future<R, Fut>(
	context: Option<crate::i18n::I18nContext>,
	future: Fut,
) -> R
where
	Fut: std::future::Future<Output = R>,
{
	let mut future = std::pin::pin!(future);
	std::future::poll_fn(move |cx| {
		if let Some(context) = context.as_ref() {
			let _guard = crate::i18n::provide_i18n_context(context.clone());
			future.as_mut().poll(cx)
		} else {
			future.as_mut().poll(cx)
		}
	})
	.await
}

impl Default for SsrRenderer {
	fn default() -> Self {
		Self::new()
	}
}

impl SsrRenderer {
	/// Creates a new renderer with default options.
	pub fn new() -> Self {
		Self {
			options: SsrOptions::default(),
			#[cfg(feature = "i18n")]
			i18n_context: None,
			state: SsrState::new(),
			rendered_head: None,
			hydration_marker_counter: 0,
			suspense_boundary_counter: 0,
			marker_resource_context: Rc::new(RefCell::new(SsrResourceContext::new(
				SsrOptions::default().resource_timeout,
			))),
			marker_id_counter: Rc::new(Cell::new(0)),
		}
	}

	/// Creates a renderer with custom options.
	pub fn with_options(options: impl IntoSsrRendererConfig) -> Self {
		let config = options.into_ssr_renderer_config();
		let resource_timeout = config.options.resource_timeout;
		Self {
			options: config.options,
			#[cfg(feature = "i18n")]
			i18n_context: config.i18n_context,
			state: SsrState::new(),
			rendered_head: None,
			hydration_marker_counter: 0,
			suspense_boundary_counter: 0,
			marker_resource_context: Rc::new(RefCell::new(SsrResourceContext::new(
				resource_timeout,
			))),
			marker_id_counter: Rc::new(Cell::new(0)),
		}
	}

	/// Returns a reference to the SSR state.
	///
	/// For streamed pages, resources resolved by later Suspense replacement
	/// chunks are serialized into the emitted stream and may not appear in this
	/// renderer snapshot after the stream has been returned.
	pub fn state(&self) -> &SsrState {
		&self.state
	}

	/// Returns a mutable reference to the SSR state.
	pub fn state_mut(&mut self) -> &mut SsrState {
		&mut self.state
	}

	/// Renders a component to an HTML string.
	pub async fn render<C: Component>(&mut self, component: &C) -> String {
		self.render_view_factory(|| component.render()).await
	}

	/// Renders an IntoPage to an HTML string.
	pub async fn render_into_page<V: IntoPage>(&mut self, view: V) -> String {
		let scope = ReactiveScope::new();
		let view = scope.enter(|| view.into_page());
		let result = self.render_view(&view).await;
		drop(scope);
		result
	}

	/// Renders a pre-built View to an HTML string.
	///
	/// This is a low-level entrypoint. Callers that construct reactive nodes
	/// before passing the view must keep its owning [`ReactiveScope`] alive.
	pub async fn render_view(&mut self, view: &Page) -> String {
		self.render_view_factory(|| view.clone()).await
	}

	/// Renders a View and syncs renderer-owned SSR state.
	pub async fn render_view_with_state(&mut self, view: &Page) -> String {
		self.render_view(view).await
	}

	fn html_lang(&self) -> String {
		#[cfg(feature = "i18n")]
		if let Some(context) = self.i18n_context.as_ref() {
			return context.locale();
		}
		self.options.lang.clone()
	}

	fn state_script_tag(&self) -> Option<String> {
		if !self.options.include_state_script {
			return None;
		}

		#[cfg(feature = "i18n")]
		let mut state = self.state.clone();
		#[cfg(not(feature = "i18n"))]
		let state = self.state.clone();
		#[cfg(feature = "i18n")]
		if let Some(context) = self.i18n_context.as_ref() {
			crate::i18n::write_i18n_ssr_state(&mut state, context);
		}

		if state.is_empty() {
			None
		} else {
			Some(state.to_script_tag())
		}
	}

	fn sync_i18n_state(&mut self) {
		#[cfg(feature = "i18n")]
		if let Some(context) = self.i18n_context.as_ref() {
			crate::i18n::write_i18n_ssr_state(&mut self.state, context);
		}
	}

	fn next_hydration_marker_id(&mut self) -> String {
		let id = self.hydration_marker_counter;
		self.hydration_marker_counter += 1;
		format!("rh-{}", id)
	}

	fn next_suspense_boundary_id(&mut self) -> String {
		let id = self.suspense_boundary_counter;
		self.suspense_boundary_counter += 1;
		format!("rh-suspense-{id}")
	}

	fn suspense_boundary_id(&mut self, node: &SuspenseNode) -> String {
		node.boundary_id()
			.map(normalize_suspense_boundary_id)
			.unwrap_or_else(|| self.next_suspense_boundary_id())
	}

	fn reset_deterministic_render_counters(&mut self) {
		self.suspense_boundary_counter = 0;
		reset_id_counter();
	}

	fn deterministic_render_snapshot(&self) -> DeterministicRenderSnapshot {
		DeterministicRenderSnapshot {
			resource_call_order_index: super::resource_context::with_active_context(|context| {
				context.borrow().call_order_index()
			}),
			suspense_boundary_counter: self.suspense_boundary_counter,
			id_counter: id_counter_snapshot(),
		}
	}

	fn restore_deterministic_render_snapshot(&mut self, snapshot: DeterministicRenderSnapshot) {
		if let Some(index) = snapshot.resource_call_order_index
			&& let Some(context) = super::resource_context::with_active_context(Rc::clone)
		{
			context.borrow_mut().set_call_order_index(index);
		}
		self.suspense_boundary_counter = snapshot.suspense_boundary_counter;
		restore_id_counter(snapshot.id_counter);
	}

	fn begin_render(&mut self, clear_resource_states: bool) {
		if clear_resource_states {
			self.state.clear_resource_states();
			self.marker_resource_context = Rc::new(RefCell::new(SsrResourceContext::new(
				self.options.resource_timeout,
			)));
			self.marker_id_counter = Rc::new(Cell::new(0));
			self.reset_deterministic_render_counters();
		}
		self.rendered_head = None;
	}

	fn should_resolve_resources(&self) -> bool {
		self.options.include_state_script
	}

	fn begin_buffered_render_pass(&mut self) {
		self.rendered_head = None;
	}

	fn record_buffered_rendered_head(&mut self, head: &Head) {
		if self.rendered_head.is_none() {
			self.rendered_head = Some(head.clone());
		}
	}

	fn current_buffered_rendered_head_or_view_head(&self, view: &Page) -> Option<Head> {
		self.rendered_head
			.clone()
			.or_else(|| view.find_topmost_head_owned())
	}

	/// Renders a component to a full HTML page.
	pub async fn render_page<C: Component>(&mut self, component: &C) -> SsrStream {
		if self.options.suspense_streaming && !self.options.minify {
			return self
				.render_page_stream_from_factory(|| component.render())
				.await;
		}

		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| component.render(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		SsrStream::from_chunks(self.wrap_in_html_with_head_and_body_tail_chunks(
			&content,
			&body_tail,
			view_head.as_ref(),
		))
	}

	/// Renders an IntoPage to a full HTML page.
	pub async fn render_page_into_page<V: IntoPage>(&mut self, view: V) -> SsrStream {
		let view = view.into_page();
		if self.options.suspense_streaming && !self.options.minify {
			return self.render_page_stream_from_factory(|| view.clone()).await;
		}

		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| view.clone(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		SsrStream::from_chunks(self.wrap_in_html_with_head_and_body_tail_chunks(
			&content,
			&body_tail,
			view_head.as_ref(),
		))
	}

	/// Renders a View to a full HTML page, using the View's attached head if present.
	///
	/// This method extracts any `Head` attached to the View using
	/// `find_topmost_head_owned()` and uses it to render the HTML `<head>`
	/// section. If no head is attached, it falls back to the head settings from
	/// `SsrOptions`.
	///
	/// # Arguments
	///
	/// * `view` - The View to render, potentially with an attached Head
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::{head, page, View, SsrRenderer};
	///
	/// let my_head = head!(|| {
	///     title { "My Page" }
	///     meta { name: "description", content: "A page" }
	/// });
	///
	/// let view = page!(|| { div { "Hello" } })().with_head(my_head);
	///
	/// let mut renderer = SsrRenderer::new();
	/// let html = renderer.render_page_with_view_head_to_string(view).await;
	/// // html contains <title>My Page</title> in the head
	/// ```
	pub async fn render_page_with_view_head(&mut self, view: Page) -> SsrStream {
		if self.options.suspense_streaming && !self.options.minify {
			return self.render_page_stream_from_factory(|| view.clone()).await;
		}

		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| view.clone(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		SsrStream::from_chunks(self.wrap_in_html_with_head_and_body_tail_chunks(
			&content,
			&body_tail,
			view_head.as_ref(),
		))
	}

	/// Renders a component to a buffered full HTML page.
	pub async fn render_page_to_string<C: Component>(&mut self, component: &C) -> String {
		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| component.render(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		self.wrap_in_html_with_head_and_body_tail(&content, &body_tail, view_head.as_ref())
	}

	/// Renders an IntoPage to a buffered full HTML page.
	pub async fn render_page_into_page_to_string<V: IntoPage>(&mut self, view: V) -> String {
		let view = view.into_page();
		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| view.clone(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		self.wrap_in_html_with_head_and_body_tail(&content, &body_tail, view_head.as_ref())
	}

	/// Renders a View to a buffered full HTML page, using attached head data.
	pub async fn render_page_with_view_head_to_string(&mut self, view: Page) -> String {
		let (view, content, body_tail) = self
			.render_view_parts_from_factory(|| view.clone(), true)
			.await;
		let view_head = self.current_buffered_rendered_head_or_view_head(&view);
		self.wrap_in_html_with_head_and_body_tail(&content, &body_tail, view_head.as_ref())
	}

	async fn render_page_stream_from_factory<F>(&mut self, mut view_factory: F) -> SsrStream
	where
		F: FnMut() -> Page,
	{
		if !self.should_resolve_resources() {
			let reactive_scope = ReactiveScope::new();
			let context = Rc::new(RefCell::new(SsrResourceContext::new(
				self.options.resource_timeout,
			)));
			#[cfg(feature = "i18n")]
			let i18n_context = self.i18n_context.clone();
			let render = scope_context(Rc::clone(&context), async move {
				self.begin_render(true);
				let render_start = self.deterministic_render_snapshot();
				let view = reactive_scope.enter(&mut view_factory);
				let mut boundaries = Vec::new();
				self.restore_deterministic_render_snapshot(render_start);
				self.begin_buffered_render_pass();
				let content = self.render_stream_shell_page(&view, &mut boundaries).await;
				let view_head = self.current_buffered_rendered_head_or_view_head(&view);
				self.sync_i18n_state();
				SsrStream::from_chunks(self.wrap_in_html_with_head_and_body_tail_chunks(
					&content,
					"",
					view_head.as_ref(),
				))
			});
			#[cfg(feature = "i18n")]
			let render = with_i18n_context_future(i18n_context, render);
			return scope_id_counter(render).await;
		}

		let reactive_scope = ReactiveScope::new();
		let context = Rc::new(RefCell::new(SsrResourceContext::new(
			self.options.resource_timeout,
		)));
		let id_counter = Rc::new(Cell::new(0));
		let scoped_id_counter = Rc::clone(&id_counter);
		#[cfg(feature = "i18n")]
		let i18n_context = self.i18n_context.clone();
		let render = scope_context(Rc::clone(&context), async move {
			self.begin_render(true);
			let render_start = self.deterministic_render_snapshot();
			let discovery_view = reactive_scope.enter(&mut view_factory);

			let _ = self
				.render_async_page(&discovery_view, AsyncRenderMode::Discovery)
				.await;
			drop(discovery_view);

			resolve_external_resources(&context).await;
			let (view, content, boundaries) = loop {
				self.restore_deterministic_render_snapshot(render_start);
				self.begin_buffered_render_pass();

				let view = reactive_scope.enter(&mut view_factory);
				let mut boundaries = Vec::new();
				let content = self.render_stream_shell_page(&view, &mut boundaries).await;

				if !context.borrow().has_pending_external() {
					break (view, content, boundaries);
				}

				drop(view);
				resolve_external_resources(&context).await;
			};
			let view_head = self.current_buffered_rendered_head_or_view_head(&view);
			self.add_resolved_resources_to_state(&context);
			self.sync_i18n_state();

			let shell = self.wrap_in_html_shell(&content, view_head.as_ref());
			let boundary_futures = suspense_boundary_futures(
				&context,
				boundaries,
				#[cfg(feature = "i18n")]
				self.i18n_context.clone(),
			);

			let runtime = SuspenseStreamRuntime {
				renderer: self.clone(),
				reactive_scope,
				context,
				id_counter,
				boundaries: boundary_futures,
				ready_boundaries: VecDeque::new(),
			};

			SsrStream::from_stream(stream::unfold(
				SuspenseStreamState::Shell { shell, runtime },
				|state| async move {
					match state {
						SuspenseStreamState::Shell { shell, runtime } => Some((
							SsrChunk::Html(shell),
							SuspenseStreamState::Boundaries(runtime),
						)),
						SuspenseStreamState::Boundaries(mut runtime) => {
							loop {
								let Some((boundary, resolved)) =
									runtime.ready_boundaries.pop_front()
								else {
									let Some(resolved_boundaries) = runtime.boundaries.next().await
									else {
										break;
									};
									runtime.ready_boundaries.extend(resolved_boundaries);
									continue;
								};

								if !resolved {
									continue;
								}

								#[cfg(feature = "i18n")]
								let i18n_context = runtime.renderer.i18n_context.clone();
								let replacement = scope_id_counter_with(
									Rc::clone(&runtime.id_counter),
									scope_context(Rc::clone(&runtime.context), async {
										if boundary.node.is_pending() {
											return None;
										}
										let (replacement, nested_boundaries) = loop {
											runtime.renderer.restore_deterministic_render_snapshot(
												boundary.boundary_start,
											);
											let boundary_guard = enter_boundary(
												&runtime.context,
												boundary.boundary_id.clone(),
											);
											let replacement_page = runtime
												.reactive_scope
												.enter(|| boundary.node.render_content());
											let mut nested_boundaries = Vec::new();
											let replacement = runtime
												.renderer
												.render_stream_shell_page(
													&replacement_page,
													&mut nested_boundaries,
												)
												.await;
											drop(boundary_guard);

											let has_pending_boundary_resource = runtime
												.context
												.borrow()
												.has_pending_for_boundary(&boundary.boundary_id);
											let has_pending_external_resource =
												runtime.context.borrow().has_pending_external();
											if !has_pending_boundary_resource
												&& !has_pending_external_resource
											{
												break (replacement, nested_boundaries);
											}

											drop(replacement_page);
											if has_pending_boundary_resource {
												resolve_boundary_resources(
													&runtime.context,
													&boundary.boundary_id,
												)
												.await;
											}
											if has_pending_external_resource {
												resolve_external_resources(&runtime.context).await;
											}
										};
										runtime
											.renderer
											.add_resolved_resources_to_state(&runtime.context);
										runtime.renderer.sync_i18n_state();
										for future in suspense_boundary_futures(
											&runtime.context,
											nested_boundaries,
											#[cfg(feature = "i18n")]
											runtime.renderer.i18n_context.clone(),
										) {
											runtime.boundaries.push(future);
										}
										Some(replacement)
									}),
								);
								#[cfg(feature = "i18n")]
								let replacement = with_i18n_context_future(i18n_context, replacement);
								let replacement = replacement.await;

								let Some(replacement) = replacement else {
									continue;
								};

								let chunk = runtime.renderer.render_suspense_replacement(
									&boundary.boundary_id,
									replacement,
								);
								return Some((
									SsrChunk::Html(chunk),
									SuspenseStreamState::Boundaries(runtime),
								));
							}

							runtime
								.renderer
								.add_resolved_resources_to_state(&runtime.context);
							runtime.renderer.sync_i18n_state();
							Some((
								SsrChunk::Html(runtime.renderer.wrap_in_html_suffix()),
								SuspenseStreamState::Done,
							))
						}
						SuspenseStreamState::Done => None,
					}
				},
			))
		});
		#[cfg(feature = "i18n")]
		let render = with_i18n_context_future(i18n_context, render);
		scope_id_counter_with(scoped_id_counter, render).await
	}

	async fn render_view_factory<F>(&mut self, view_factory: F) -> String
	where
		F: FnMut() -> Page,
	{
		let (_, content, body_tail) = self
			.render_view_parts_from_factory(view_factory, true)
			.await;
		format!("{content}{body_tail}")
	}

	async fn render_view_factory_preserving_resource_state<F>(&mut self, view_factory: F) -> String
	where
		F: FnMut() -> Page,
	{
		let (_, content, body_tail) = self
			.render_view_parts_from_factory(view_factory, false)
			.await;
		format!("{content}{body_tail}")
	}

	async fn render_view_parts_from_factory<F>(
		&mut self,
		mut view_factory: F,
		clear_resource_states: bool,
	) -> (Page, String, String)
	where
		F: FnMut() -> Page,
	{
		let reactive_scope = ReactiveScope::new();
		let context = if clear_resource_states {
			Rc::new(RefCell::new(SsrResourceContext::new(
				self.options.resource_timeout,
			)))
		} else {
			Rc::clone(&self.marker_resource_context)
		};
		let marker_id_counter =
			(!clear_resource_states).then(|| Rc::clone(&self.marker_id_counter));
		#[cfg(feature = "i18n")]
		let i18n_context = self.i18n_context.clone();
		let render = scope_context(Rc::clone(&context), async move {
			self.begin_render(clear_resource_states);
			let render_start = self.deterministic_render_snapshot();
			if !self.should_resolve_resources() {
				self.restore_deterministic_render_snapshot(render_start);
				let view = reactive_scope.enter(&mut view_factory);
				self.begin_buffered_render_pass();
				let content = self
					.render_async_page(&view, AsyncRenderMode::Buffered)
					.await;
				self.sync_i18n_state();
				return (view, content, String::new());
			}

			let discovery_view = reactive_scope.enter(&mut view_factory);

			let _ = self
				.render_async_page(&discovery_view, AsyncRenderMode::Discovery)
				.await;
			drop(discovery_view);

			resolve_external_resources(&context).await;
			resolve_pending_resources(&context).await;

			loop {
				self.restore_deterministic_render_snapshot(render_start);
				self.begin_buffered_render_pass();

				let view = reactive_scope.enter(&mut view_factory);
				let content = self
					.render_async_page(&view, AsyncRenderMode::Buffered)
					.await;

				if !context.borrow().has_pending() {
					self.add_resolved_resources_to_state(&context);
					self.sync_i18n_state();
					return (view, content, String::new());
				}

				drop(view);
				resolve_pending_resources(&context).await;
			}
		});
		#[cfg(feature = "i18n")]
		let render = with_i18n_context_future(i18n_context, render);
		if let Some(marker_id_counter) = marker_id_counter {
			scope_id_counter_with(marker_id_counter, render).await
		} else {
			scope_id_counter(render).await
		}
	}

	fn add_resolved_resources_to_state(&mut self, context: &Rc<RefCell<SsrResourceContext>>) {
		for (id, value) in context.borrow().resolved_resources() {
			self.state.add_resource_state(id, value.clone());
		}
	}

	fn render_stream_shell_page<'a>(
		&'a mut self,
		view: &'a Page,
		boundaries: &'a mut Vec<PendingSuspenseBoundary>,
	) -> LocalBoxFuture<'a, String> {
		Box::pin(async move {
			match view {
				Page::Element(el) => {
					let mut html = String::new();
					html.push('<');
					html.push_str(el.tag_name());

					for (name, value) in el.attrs() {
						let name_str = name.as_ref();
						if BOOLEAN_ATTRS.contains(&name_str) && !is_boolean_attr_truthy(value) {
							continue;
						}

						html.push(' ');
						html.push_str(name.as_ref());
						html.push_str("=\"");
						html.push_str(&html_escape(value));
						html.push('"');
					}

					if el.is_void() {
						html.push_str(" />");
					} else {
						html.push('>');
						for child in el.child_views() {
							html.push_str(&self.render_stream_shell_page(child, boundaries).await);
						}
						html.push_str("</");
						html.push_str(el.tag_name());
						html.push('>');
					}

					html
				}
				Page::Text(text) => html_escape(text),
				Page::Fragment(children) => {
					let mut html = String::new();
					for child in children {
						html.push_str(&self.render_stream_shell_page(child, boundaries).await);
					}
					html
				}
				Page::KeyedFragment(children) => {
					let mut html = String::new();
					for (_, child) in children {
						html.push_str(&self.render_stream_shell_page(child, boundaries).await);
					}
					html
				}
				Page::Empty => String::new(),
				Page::WithHead { view, head } => {
					self.record_buffered_rendered_head(head);
					self.render_stream_shell_page(view, boundaries).await
				}
				Page::ReactiveIf(reactive_if) => {
					let branch = if reactive_if.condition() {
						reactive_if.then_view()
					} else {
						reactive_if.else_view()
					};
					self.render_stream_shell_page(&branch, boundaries).await
				}
				Page::Reactive(reactive) => {
					let rendered = reactive.render();
					self.render_stream_shell_page(&rendered, boundaries).await
				}
				Page::Suspense(node) => {
					let boundary_id = self.suspense_boundary_id(node);

					if let Some(context) = super::resource_context::with_active_context(Rc::clone) {
						context.borrow_mut().assign_resources_to_boundary(
							node.tracked_resource_ids(),
							&boundary_id,
						);
					}

					let boundary_start = self.deterministic_render_snapshot();
					let boundary_guard = super::resource_context::with_active_context(|context| {
						enter_boundary(context, boundary_id.clone())
					});
					let content_page = node.render_content();
					let content = self
						.render_stream_shell_page(&content_page, boundaries)
						.await;

					drop(boundary_guard);

					let has_pending = super::resource_context::with_active_context(|context| {
						context.borrow().has_pending_for_boundary(&boundary_id)
					})
					.unwrap_or(false);

					if has_pending || node.is_pending() {
						self.restore_deterministic_render_snapshot(boundary_start);
						let fallback_page = node.render_fallback();
						let fallback = self
							.render_stream_shell_page(&fallback_page, boundaries)
							.await;
						if has_pending {
							boundaries.push(PendingSuspenseBoundary {
								boundary_id: boundary_id.clone(),
								node: node.clone(),
								boundary_start,
							});
						}
						self.render_suspense_fallback(&boundary_id, fallback)
					} else {
						content
					}
				}
				Page::Deferred(node) => {
					let content = node.render_content();
					self.render_stream_shell_page(&content, boundaries).await
				}
				Page::Outlet(outlet) => {
					if let Some(child) = outlet.child() {
						self.render_stream_shell_page(child, boundaries).await
					} else {
						String::new()
					}
				}
			}
		})
	}

	fn render_async_page<'a>(
		&'a mut self,
		view: &'a Page,
		mode: AsyncRenderMode,
	) -> LocalBoxFuture<'a, String> {
		Box::pin(async move {
			match view {
				Page::Element(el) => {
					let mut html = String::new();
					html.push('<');
					html.push_str(el.tag_name());

					for (name, value) in el.attrs() {
						let name_str = name.as_ref();
						if BOOLEAN_ATTRS.contains(&name_str) && !is_boolean_attr_truthy(value) {
							continue;
						}

						html.push(' ');
						html.push_str(name.as_ref());
						html.push_str("=\"");
						html.push_str(&html_escape(value));
						html.push('"');
					}

					if el.is_void() {
						html.push_str(" />");
					} else {
						html.push('>');
						for child in el.child_views() {
							html.push_str(&self.render_async_page(child, mode).await);
						}
						html.push_str("</");
						html.push_str(el.tag_name());
						html.push('>');
					}

					html
				}
				Page::Text(text) => html_escape(text),
				Page::Fragment(children) => {
					let mut html = String::new();
					for child in children {
						html.push_str(&self.render_async_page(child, mode).await);
					}
					html
				}
				Page::KeyedFragment(children) => {
					let mut html = String::new();
					for (_, child) in children {
						html.push_str(&self.render_async_page(child, mode).await);
					}
					html
				}
				Page::Empty => String::new(),
				Page::WithHead { view, head } => {
					if !matches!(mode, AsyncRenderMode::Discovery) {
						self.record_buffered_rendered_head(head);
					}
					self.render_async_page(view, mode).await
				}
				Page::ReactiveIf(reactive_if) => {
					let branch = if reactive_if.condition() {
						reactive_if.then_view()
					} else {
						reactive_if.else_view()
					};
					self.render_async_page(&branch, mode).await
				}
				Page::Reactive(reactive) => {
					let rendered = reactive.render();
					self.render_async_page(&rendered, mode).await
				}
				Page::Suspense(node) => {
					let boundary_id = self.suspense_boundary_id(node);

					if let Some(context) = super::resource_context::with_active_context(Rc::clone) {
						context.borrow_mut().assign_resources_to_boundary(
							node.tracked_resource_ids(),
							&boundary_id,
						);
					}

					let boundary_start = self.deterministic_render_snapshot();
					let boundary_guard = super::resource_context::with_active_context(|context| {
						enter_boundary(context, boundary_id.clone())
					});
					let content_page = node.render_content();
					let content = self.render_async_page(&content_page, mode).await;
					let boundary_end_index =
						super::resource_context::with_active_context(|context| {
							context.borrow().call_order_index()
						});

					drop(boundary_guard);

					let has_pending = super::resource_context::with_active_context(|context| {
						context.borrow().has_pending_for_boundary(&boundary_id)
					})
					.unwrap_or(false);

					if matches!(mode, AsyncRenderMode::Discovery) {
						if has_pending || node.is_pending() {
							self.restore_deterministic_render_snapshot(boundary_start);
						}
						return content;
					}

					if has_pending || node.is_pending() {
						self.restore_deterministic_render_snapshot(boundary_start);
						let fallback_page = node.render_fallback();
						let fallback = self
							.render_async_page(&fallback_page, AsyncRenderMode::Buffered)
							.await;

						if has_pending
							&& let Some(context) =
								super::resource_context::with_active_context(Rc::clone)
						{
							if !self.should_resolve_resources() {
								return self.render_suspense_fallback(&boundary_id, fallback);
							}
							if context.borrow().has_pending_external() {
								return self.render_suspense_fallback(&boundary_id, fallback);
							}
							let boundary_resolved =
								resolve_boundary_resources(&context, &boundary_id).await;
							if !boundary_resolved {
								return self.render_suspense_fallback(&boundary_id, fallback);
							}
							self.restore_deterministic_render_snapshot(boundary_start);
							if node.is_pending() {
								return self.render_suspense_fallback(&boundary_id, fallback);
							}
						} else {
							return self.render_suspense_fallback(&boundary_id, fallback);
						}

						let boundary_guard =
							super::resource_context::with_active_context(|context| {
								enter_boundary(context, boundary_id.clone())
							});
						let replacement_page = node.render_content();
						let replacement = self
							.render_async_page(&replacement_page, AsyncRenderMode::Buffered)
							.await;
						drop(boundary_guard);
						if let Some(index) = boundary_end_index
							&& let Some(context) =
								super::resource_context::with_active_context(Rc::clone)
						{
							context.borrow_mut().set_call_order_index(index);
						}

						replacement
					} else {
						content
					}
				}
				Page::Deferred(node) => {
					let content = node.render_content();
					self.render_async_page(&content, mode).await
				}
				Page::Outlet(outlet) => {
					if let Some(child) = outlet.child() {
						self.render_async_page(child, mode).await
					} else {
						String::new()
					}
				}
			}
		})
	}

	fn render_suspense_fallback(&self, id: &str, fallback: String) -> String {
		let fallback = if fallback.contains("data-rh-suspense=\"pending\"") {
			fallback
		} else {
			format!(r#"<div data-rh-suspense="pending">{fallback}</div>"#)
		};
		format!("<!--rh-suspense-start:{id}-->{fallback}<!--rh-suspense-end:{id}-->")
	}

	fn render_suspense_replacement(&self, id: &str, content: String) -> String {
		let escaped_id = html_escape(id);
		let nonce = self
			.options
			.script_nonce
			.as_ref()
			.map(|value| format!(" nonce=\"{}\"", html_escape(value)))
			.unwrap_or_default();
		let id_json = serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string());
		let safe_id_json = escape_json_for_script(&id_json);
		format!(
			"<template data-rh-suspense-chunk=\"{escaped_id}\">{content}</template><script{nonce}>(function(id){{var template=document.querySelector('template[data-rh-suspense-chunk=\"'+id+'\"]');if(!template)return;var start=document.createComment('');var walker=document.createTreeWalker(document.body,128);while(start=walker.nextNode()){{if(start.nodeValue==='rh-suspense-start:'+id)break;}}if(!start)return;var end=start;while(end=end.nextSibling){{if(end.nodeType===8&&end.nodeValue==='rh-suspense-end:'+id)break;}}if(!end)return;var fragment=template.content.cloneNode(true);var parent=start.parentNode;var node=start.nextSibling;while(node&&node!==end){{var next=node.nextSibling;parent.removeChild(node);node=next;}}parent.insertBefore(fragment,end);template.remove();}})({safe_id_json});</script>"
		)
	}

	fn write_html_head(&self, html: &mut String, view_head: Option<&Head>) {
		let view_head = view_head.map(Head::deduplicated);
		let mut seen_head_entries = BTreeSet::new();

		html.push_str("<!DOCTYPE html>\n");
		html.push_str(&format!(
			"<html lang=\"{}\">\n",
			html_escape(&self.html_lang())
		));
		html.push_str("<head>\n");
		push_unique_head_entry(html, &mut seen_head_entries, "<meta charset=\"UTF-8\">");
		push_unique_head_entry(
			html,
			&mut seen_head_entries,
			"<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">",
		);

		if let Some(head) = view_head.as_ref() {
			if let Some(ref title) = head.title {
				html.push_str(&format!("<title>{}</title>\n", html_escape(title)));
			}
			for meta in &head.meta_tags {
				push_unique_head_entry(html, &mut seen_head_entries, meta.to_html());
			}
			for link in &head.links {
				push_unique_head_entry(html, &mut seen_head_entries, link.to_html());
			}
			for style in &head.styles {
				push_unique_head_entry(html, &mut seen_head_entries, style.to_html());
			}
			for script in &head.scripts {
				push_unique_head_entry(html, &mut seen_head_entries, script.to_html());
			}
		}

		if let Some(ref token) = self.options.csrf_token {
			html.push_str(&format!(
				"<meta name=\"csrf-token\" content=\"{}\">\n",
				html_escape(token)
			));
		}

		html.push_str("</head>\n");
	}

	fn wrap_in_html_shell(&self, content: &str, view_head: Option<&Head>) -> String {
		let mut shell = String::with_capacity(content.len() + 1024);

		self.write_html_head(&mut shell, view_head);
		shell.push_str("<body>\n<div id=\"app\">");
		shell.push_str(content);
		shell.push_str("</div>\n");
		shell
	}

	fn wrap_in_html_suffix(&self) -> String {
		let mut suffix = String::new();
		if let Some(ref auth_data) = self.options.auth_data
			&& let Ok(json) = serde_json::to_string(auth_data)
		{
			let safe_json = escape_json_for_script(&json);
			suffix.push_str(&format!(
				"<script id=\"auth-data\" type=\"application/json\">{}</script>\n",
				safe_json
			));
		}

		if let Some(state_script) = self.state_script_tag() {
			suffix.push_str(&state_script);
			suffix.push('\n');
		}
		suffix.push_str("</body>\n</html>");
		suffix
	}

	/// Wraps content in a full HTML document with View's head elements.
	///
	/// Head elements (title, meta tags, CSS links, JS scripts) are sourced
	/// from the View's attached Head. Use the `head!` macro to define
	/// head elements.
	///
	/// # Arguments
	///
	/// * `content` - The rendered body content
	/// * `view_head` - Optional head extracted from a View
	fn wrap_in_html_with_head_and_body_tail_chunks(
		&self,
		content: &str,
		body_tail: &str,
		view_head: Option<&Head>,
	) -> Vec<SsrChunk> {
		if self.options.minify {
			return vec![SsrChunk::Html(self.wrap_in_html_with_head_and_body_tail(
				content, body_tail, view_head,
			))];
		}

		let mut shell = String::with_capacity(content.len() + 1024);

		self.write_html_head(&mut shell, view_head);
		shell.push_str("<body>\n<div id=\"app\">");
		shell.push_str(content);
		shell.push_str("</div>\n");

		let mut suffix = String::new();
		if let Some(ref auth_data) = self.options.auth_data
			&& let Ok(json) = serde_json::to_string(auth_data)
		{
			let safe_json = escape_json_for_script(&json);
			suffix.push_str(&format!(
				"<script id=\"auth-data\" type=\"application/json\">{}</script>\n",
				safe_json
			));
		}

		if let Some(state_script) = self.state_script_tag() {
			suffix.push_str(&state_script);
			suffix.push('\n');
		}
		suffix.push_str("</body>\n</html>");

		let mut chunks = vec![SsrChunk::Html(shell)];
		if !body_tail.is_empty() {
			chunks.push(SsrChunk::Html(body_tail.to_string()));
		}
		chunks.push(SsrChunk::Html(suffix));
		chunks
	}

	fn wrap_in_html_with_head_and_body_tail(
		&self,
		content: &str,
		body_tail: &str,
		view_head: Option<&Head>,
	) -> String {
		let mut html = String::with_capacity(content.len() + 1024);

		self.write_html_head(&mut html, view_head);

		// Body section
		html.push_str("<body>\n");
		html.push_str("<div id=\"app\">");
		html.push_str(content);
		html.push_str("</div>\n");
		html.push_str(body_tail);

		// Auth data script (if provided)
		// Note: We escape </script> sequences to prevent XSS attacks where
		// user-controlled data (like username) could break out of the script context
		if let Some(ref auth_data) = self.options.auth_data
			&& let Ok(json) = serde_json::to_string(auth_data)
		{
			let safe_json = escape_json_for_script(&json);
			html.push_str(&format!(
				"<script id=\"auth-data\" type=\"application/json\">{}</script>\n",
				safe_json
			));
		}

		// SSR state script (if enabled)
		if let Some(state_script) = self.state_script_tag() {
			html.push_str(&state_script);
			html.push('\n');
		}

		html.push_str("</body>\n");
		html.push_str("</html>");

		if self.options.minify {
			minify_html(&html)
		} else {
			html
		}
	}

	/// Wraps content in a full HTML document.
	///
	/// This method creates a minimal HTML document without head elements.
	/// Use `render_page_with_view_head` with the `head!` macro for pages
	/// that require title, meta tags, CSS, or JS.
	pub fn wrap_in_html(&self, content: &str) -> String {
		let mut html = String::with_capacity(content.len() + 1024);
		self.write_html_head(&mut html, None);

		// Body section
		html.push_str("<body>\n");

		// Main content
		html.push_str("<div id=\"app\">");
		html.push_str(content);
		html.push_str("</div>\n");

		// Auth data script (if provided)
		// Note: We escape </script> sequences to prevent XSS attacks where
		// user-controlled data (like username) could break out of the script context
		if let Some(ref auth_data) = self.options.auth_data
			&& let Ok(json) = serde_json::to_string(auth_data)
		{
			let safe_json = escape_json_for_script(&json);
			html.push_str(&format!(
				"<script id=\"auth-data\" type=\"application/json\">{}</script>\n",
				safe_json
			));
		}

		// SSR state script (if enabled)
		if let Some(state_script) = self.state_script_tag() {
			html.push_str(&state_script);
			html.push('\n');
		}

		html.push_str("</body>\n");
		html.push_str("</html>");

		if self.options.minify {
			minify_html(&html)
		} else {
			html
		}
	}

	/// Renders a component with hydration marker.
	pub async fn render_with_marker<C: Component>(&mut self, component: &C) -> String {
		let content = self
			.render_view_factory_preserving_resource_state(|| component.render())
			.await;

		if self.options.include_hydration_markers {
			let marker = HydrationMarker {
				id: self.next_hydration_marker_id(),
				component_name: Some(C::name().to_string()),
				props: None,
				strategy: HydrationStrategy::default(),
			};
			format!("<div {}>{}</div>", marker.to_attr_string(), content)
		} else {
			content
		}
	}
}

/// Simple HTML escape function.
fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
}

/// Escapes JSON content for safe embedding in HTML script tags.
///
/// This function prevents XSS attacks by escaping `</script>` sequences
/// that could break out of the script context. The escaping is done by
/// replacing `</` with `<\/`, which is safe because:
/// 1. JavaScript string literals interpret `<\/` as `</`
/// 2. HTML parsers don't recognize `<\/script>` as a closing tag
///
/// # Security Note
///
/// When embedding JSON data in `<script>` tags, the `</script>` sequence
/// must be escaped because HTML parsers don't understand JavaScript string
/// context - they will see `</script>` and close the tag, allowing XSS.
fn escape_json_for_script(json: &str) -> String {
	json.replace("</", "<\\/")
}

fn is_comment_safe_suspense_id(id: &str) -> bool {
	!id.is_empty()
		&& !id.contains("--")
		&& !id.ends_with('-')
		&& id.bytes().all(|byte| {
			matches!(
				byte,
				b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b':' | b'.'
			)
		})
}

fn normalize_suspense_boundary_id(id: &str) -> String {
	if is_comment_safe_suspense_id(id)
		&& !id.starts_with("rh-suspense-id-")
		&& !is_generated_suspense_boundary_id(id)
	{
		return id.to_string();
	}

	let mut normalized = String::from("rh-suspense-id-");
	for byte in id.as_bytes() {
		let _ = write!(&mut normalized, "{byte:02x}");
	}
	normalized
}

fn is_generated_suspense_boundary_id(id: &str) -> bool {
	id.strip_prefix("rh-suspense-").is_some_and(|suffix| {
		!suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
	})
}

fn push_unique_head_entry(
	html: &mut String,
	seen: &mut BTreeSet<String>,
	entry: impl Into<String>,
) {
	let entry = entry.into();
	if seen.contains(&entry) {
		return;
	}
	html.push_str(&entry);
	html.push('\n');
	seen.insert(entry);
}

/// Maximum input size for HTML minification (1 MiB).
///
/// Inputs exceeding this limit are returned unmodified to prevent
/// denial-of-service via excessively large payloads.
const MINIFY_HTML_MAX_INPUT_SIZE: usize = 1024 * 1024;

/// Tag names whose content must be preserved verbatim during minification.
///
/// - `pre`, `textarea`: whitespace is semantically significant
/// - `script`, `style`: whitespace removal can break code/selectors
const PRESERVED_TAGS: [&str; 4] = ["pre", "textarea", "script", "style"];

/// Simple HTML minification (removes extra whitespace).
///
/// Returns the input unmodified when its byte length exceeds
/// `MINIFY_HTML_MAX_INPUT_SIZE` (1MB) to prevent denial-of-service attacks.
///
/// Whitespace inside `<pre>`, `<textarea>`, `<script>`, and `<style>` blocks
/// is preserved.
fn minify_html(html: &str) -> String {
	if html.len() > MINIFY_HTML_MAX_INPUT_SIZE {
		return html.to_string();
	}

	let mut result = String::with_capacity(html.len());
	let mut prev_was_whitespace = false;
	// When inside a preserved tag, holds the tag name for closing-tag matching
	let mut preserved_tag: Option<&str> = None;
	let mut chars = html.char_indices().peekable();

	while let Some((byte_pos, c)) = chars.next() {
		let remaining = &html[byte_pos..];

		// Detect opening preserved tag (case-insensitive, e.g. <pre>, <PRE>, <Pre class="...">)
		if preserved_tag.is_none() && c == '<' {
			for tag in &PRESERVED_TAGS {
				// Bounded, allocation-free case-insensitive comparison:
				// only inspect `<` + tag length + 1 char instead of lowercasing all remaining input
				let open_len = 1 + tag.len(); // "<" + tag name
				if remaining.len() >= open_len
					&& remaining[1..open_len].eq_ignore_ascii_case(tag)
					&& remaining[open_len..]
						.starts_with(|ch: char| ch == '>' || ch.is_ascii_whitespace())
				{
					preserved_tag = Some(tag);
					break;
				}
			}
		}

		// Detect closing tag for the currently preserved tag (case-insensitive)
		if let Some(tag) = preserved_tag {
			let close = format!("</{tag}>");
			if c == '<'
				&& remaining.len() >= close.len()
				&& remaining[..close.len()].eq_ignore_ascii_case(&close)
			{
				// Push the original-cased closing tag from the source
				result.push_str(&remaining[..close.len()]);
				// Skip the remaining chars of the closing tag (we already consumed '<')
				for _ in 0..close.len() - 1 {
					chars.next();
				}
				preserved_tag = None;
				prev_was_whitespace = false;
				continue;
			}
		}

		if preserved_tag.is_some() {
			result.push(c);
		} else if c.is_whitespace() {
			if !prev_was_whitespace {
				result.push(' ');
				prev_was_whitespace = true;
			}
		} else {
			result.push(c);
			prev_was_whitespace = false;
		}
	}

	result
}

/// Helper function for simple component rendering.
// Allow dead_code: convenience function for internal module use and tests
#[allow(dead_code)]
pub(super) async fn render<C: Component>(component: &C) -> String {
	let mut renderer = SsrRenderer::new();
	renderer.render(component).await
}

/// Helper function for rendering to a full HTML page.
// Allow dead_code: convenience function for internal module use and tests
#[allow(dead_code)]
pub(super) async fn render_page<C: Component>(component: &C, options: SsrOptions) -> String {
	let mut renderer = SsrRenderer::with_options(options);
	renderer.render_page_to_string(component).await
}

// Phase 2-B Tests: SsrOptions Extension

#[test]
fn test_ssr_options_partial_hydration_default() {
	let opts = SsrOptions::default();
	assert!(!opts.enable_partial_hydration);
	assert_eq!(opts.default_hydration_strategy, HydrationStrategy::Full);
	assert_eq!(opts.resource_timeout, Duration::from_secs(2));
	assert!(opts.suspense_streaming);
	assert_eq!(opts.script_nonce, None);
}

#[test]
fn test_ssr_options_partial_hydration_builder() {
	let opts = SsrOptions::new()
		.partial_hydration(true)
		.default_strategy(HydrationStrategy::Island);

	assert!(opts.enable_partial_hydration);
	assert_eq!(opts.default_hydration_strategy, HydrationStrategy::Island);
}

#[test]
fn test_ssr_options_islands_only() {
	let opts = SsrOptions::new().islands_only();

	assert!(opts.enable_partial_hydration);
	assert_eq!(opts.default_hydration_strategy, HydrationStrategy::Island);
}

#[test]
fn test_ssr_options_default_strategy_static() {
	let opts = SsrOptions::new().default_strategy(HydrationStrategy::Static);

	assert!(!opts.enable_partial_hydration);
	assert_eq!(opts.default_hydration_strategy, HydrationStrategy::Static);
}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::component::PageElement;
	use rstest::rstest;

	struct TestComponent {
		message: String,
	}

	impl Component for TestComponent {
		fn render(&self) -> Page {
			PageElement::new("div")
				.attr("class", "test")
				.child(self.message.clone())
				.into_page()
		}

		fn name() -> &'static str {
			"TestComponent"
		}
	}

	#[test]
	fn normalize_suspense_boundary_id_reserves_generated_namespace() {
		let unsafe_id = normalize_suspense_boundary_id("--");
		let generated_id = normalize_suspense_boundary_id("rh-suspense-0");
		let safe_generated_prefix_id = normalize_suspense_boundary_id("rh-suspense-id-2d2d");

		assert_eq!(unsafe_id, "rh-suspense-id-2d2d");
		assert_eq!(generated_id, "rh-suspense-id-72682d73757370656e73652d30");
		assert_ne!(unsafe_id, safe_generated_prefix_id);
		assert!(safe_generated_prefix_id.starts_with("rh-suspense-id-"));
	}

	#[test]
	fn test_ssr_options_default() {
		let opts = SsrOptions::default();
		assert!(opts.include_hydration_markers);
		assert!(!opts.minify);
		assert_eq!(opts.lang, "en");
		assert_eq!(opts.resource_timeout, Duration::from_secs(2));
		assert!(opts.suspense_streaming);
	}

	#[tokio::test]
	async fn test_ssr_renderer_render() {
		let component = TestComponent {
			message: "Hello".to_string(),
		};
		let mut renderer = SsrRenderer::new();
		let html = renderer.render(&component).await;
		assert_eq!(html, "<div class=\"test\">Hello</div>");
	}

	#[tokio::test]
	async fn ssr_render_creates_isolated_reactive_scopes() {
		use crate::reactive::Signal;

		struct ScopedCounter;

		impl Component for ScopedCounter {
			fn render(&self) -> Page {
				let count = Signal::new(1);
				Page::text(count.get().to_string())
			}

			fn name() -> &'static str {
				"ScopedCounter"
			}
		}

		let mut renderer = SsrRenderer::new();

		assert_eq!(renderer.render(&ScopedCounter).await, "1");
		assert_eq!(renderer.render(&ScopedCounter).await, "1");
	}

	#[tokio::test]
	async fn test_ssr_renderer_with_csrf() {
		let component = TestComponent {
			message: "Secure".to_string(),
		};
		let opts = SsrOptions::new().csrf("test-token-123");
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page_to_string(&component).await;

		assert!(html.contains("csrf-token"));
		assert!(html.contains("test-token-123"));
	}

	#[tokio::test]
	async fn test_ssr_renderer_with_auth() {
		let component = TestComponent {
			message: "Auth".to_string(),
		};
		let auth = AuthData::authenticated("1", "testuser");
		let opts = SsrOptions::new().auth(auth);
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page_to_string(&component).await;

		assert!(html.contains("auth-data"));
		assert!(html.contains("testuser"));
	}

	#[tokio::test]
	async fn test_ssr_renderer_with_marker() {
		let component = TestComponent {
			message: "Hydrate".to_string(),
		};
		let mut renderer = SsrRenderer::new();
		let html = renderer.render_with_marker(&component).await;

		assert!(html.contains("data-rh-id"));
		assert!(html.contains("data-rh-component=\"TestComponent\""));
	}

	#[tokio::test]
	async fn test_render_helper() {
		let component = TestComponent {
			message: "Helper".to_string(),
		};
		let html = render(&component).await;
		assert_eq!(html, "<div class=\"test\">Helper</div>");
	}

	#[test]
	fn test_html_escape() {
		assert_eq!(html_escape("<script>"), "&lt;script&gt;");
		assert_eq!(html_escape("a&b"), "a&amp;b");
		assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
	}

	#[test]
	fn test_escape_json_for_script() {
		// Verify that </script> is escaped to prevent XSS
		assert_eq!(escape_json_for_script("</script>"), "<\\/script>");
		// Verify that </ is escaped in any context
		assert_eq!(
			escape_json_for_script("</script><script>alert(1)</script>"),
			"<\\/script><script>alert(1)<\\/script>"
		);
		// Normal JSON should not be modified
		assert_eq!(
			escape_json_for_script(r#"{"name":"test"}"#),
			r#"{"name":"test"}"#
		);
	}

	#[tokio::test]
	async fn test_ssr_renderer_with_auth_xss_prevention() {
		// Test that auth data with </script> in username is properly escaped
		let component = TestComponent {
			message: "Auth".to_string(),
		};
		// Simulate a malicious username that contains </script>
		let malicious_username = "</script><script>alert('xss')</script>";
		let auth = AuthData::authenticated("1", malicious_username);
		let opts = SsrOptions::new().auth(auth);
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page_to_string(&component).await;

		// Verify the auth-data script tag exists
		assert!(html.contains("auth-data"));

		// Verify that </script> sequences are escaped in the JSON
		// The raw </script> should NOT appear in the HTML output
		assert!(!html.contains("</script><script>alert"));

		// The escaped version should be present
		assert!(html.contains("<\\/script>"));
	}

	#[tokio::test]
	async fn test_ssr_renderer_with_auth_xss_prevention_wrap_in_html_with_head() {
		use crate::component::PageElement;

		// Test XSS prevention via wrap_in_html_with_head path
		struct TestPage {
			message: String,
		}

		impl Component for TestPage {
			fn render(&self) -> Page {
				PageElement::new("div")
					.child(self.message.clone())
					.into_page()
			}

			fn name() -> &'static str {
				"TestPage"
			}
		}

		let component = TestPage {
			message: "Test".to_string(),
		};
		// Simulate a malicious username
		let malicious_username = "</script><img src=x onerror=alert(1)>";
		let auth = AuthData::authenticated("1", malicious_username);
		let opts = SsrOptions::new().auth(auth);
		let mut renderer = SsrRenderer::with_options(opts);
		let view = component.render();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		// Verify that raw </script> does not appear (it should be escaped)
		assert!(!html.contains("</script><img"));
		// The escaped version should be present
		assert!(html.contains("<\\/script>"));
	}

	#[tokio::test]
	async fn test_ssr_renderer_lang_escaping() {
		// Arrange
		let component = TestComponent {
			message: "Test".to_string(),
		};
		let opts = SsrOptions::new().lang("en\" onload=\"alert(1)");
		let mut renderer = SsrRenderer::with_options(opts);

		// Act
		let html = renderer.render_page_to_string(&component).await;

		// Assert - the quote in the lang value should be escaped, preventing attribute breakout
		assert!(html.contains("&quot;"));
		// The entire malicious value should be contained within the lang attribute
		assert!(html.contains("lang=\"en&quot; onload=&quot;alert(1)\""));
	}

	#[rstest]
	#[case::pre("<pre>  hello\n  world  </pre>", "<pre>  hello\n  world  </pre>")]
	#[case::textarea(
		"<textarea>  hello\n  world  </textarea>",
		"<textarea>  hello\n  world  </textarea>"
	)]
	#[case::style(
		"<style>  .foo  {  color: red;  }  </style>",
		"<style>  .foo  {  color: red;  }  </style>"
	)]
	#[case::script(
		"<script>  var x  =  1;  </script>",
		"<script>  var x  =  1;  </script>"
	)]
	#[case::pre_with_attrs(
		"<pre class=\"code\">  spaced  </pre>",
		"<pre class=\"code\">  spaced  </pre>"
	)]
	#[case::textarea_with_attrs(
		"<textarea rows=\"5\">  multi\n  line  </textarea>",
		"<textarea rows=\"5\">  multi\n  line  </textarea>"
	)]
	#[case::surrounding_whitespace_collapsed(
		"<div>  hello  </div>  <pre>  keep  </pre>  <div>  world  </div>",
		"<div> hello </div> <pre>  keep  </pre> <div> world </div>"
	)]
	#[case::pre_uppercase("<PRE>  hello\n  world  </PRE>", "<PRE>  hello\n  world  </PRE>")]
	#[case::pre_mixed_case("<Pre>  hello\n  world  </Pre>", "<Pre>  hello\n  world  </Pre>")]
	#[case::textarea_uppercase(
		"<TEXTAREA>  multi\n  line  </TEXTAREA>",
		"<TEXTAREA>  multi\n  line  </TEXTAREA>"
	)]
	#[case::script_uppercase(
		"<SCRIPT>  var x  =  1;  </SCRIPT>",
		"<SCRIPT>  var x  =  1;  </SCRIPT>"
	)]
	#[case::style_mixed_case(
		"<Style>  .foo  {  color: red;  }  </Style>",
		"<Style>  .foo  {  color: red;  }  </Style>"
	)]
	#[case::pre_uppercase_with_attrs(
		"<PRE class=\"code\">  spaced  </PRE>",
		"<PRE class=\"code\">  spaced  </PRE>"
	)]
	fn test_minify_html_preserves_tag_content(#[case] input: &str, #[case] expected: &str) {
		// Arrange (input and expected provided by rstest cases)

		// Act
		let result = minify_html(input);

		// Assert
		assert_eq!(result, expected);
	}

	#[tokio::test]
	async fn test_ssr_renderer_lang_escaping_angle_brackets() {
		// Arrange
		let component = TestComponent {
			message: "Test".to_string(),
		};
		let opts = SsrOptions::new().lang("<script>alert(1)</script>");
		let mut renderer = SsrRenderer::with_options(opts);

		// Act
		let html = renderer.render_page_to_string(&component).await;

		// Assert - angle brackets should be escaped
		assert!(html.contains("&lt;script&gt;"));
		assert!(!html.contains("<html lang=\"<script>"));
	}
}
