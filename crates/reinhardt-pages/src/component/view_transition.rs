//! View Transition API helpers and SSR-safe boundary markers.

use std::borrow::Cow;

use crate::component::{IntoPage, Page, PageElement};

/// Result status for [`start_view_transition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewTransitionStatus {
	/// Browser View Transition API was called successfully.
	Started,
	/// Browser support was unavailable; the update still ran.
	Unsupported,
	/// The browser API failed; the update still ran when possible.
	Failed(String),
}

/// Handle returned by [`start_view_transition`].
pub struct ViewTransitionHandle {
	status: ViewTransitionStatus,
	#[cfg(wasm)]
	transition: Option<wasm_bindgen::JsValue>,
}

impl std::fmt::Debug for ViewTransitionHandle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViewTransitionHandle")
			.field("status", &self.status)
			.finish_non_exhaustive()
	}
}

impl ViewTransitionHandle {
	fn from_status(status: ViewTransitionStatus) -> Self {
		Self {
			status,
			#[cfg(wasm)]
			transition: None,
		}
	}

	#[cfg(wasm)]
	fn started(transition: wasm_bindgen::JsValue) -> Self {
		Self {
			status: ViewTransitionStatus::Started,
			transition: Some(transition),
		}
	}

	/// Returns the transition status.
	pub const fn status(&self) -> &ViewTransitionStatus {
		&self.status
	}

	/// Returns `true` when the browser View Transition API was used.
	pub fn is_started(&self) -> bool {
		matches!(self.status, ViewTransitionStatus::Started)
	}

	/// Returns `true` when the update ran without browser View Transition support.
	pub fn is_unsupported(&self) -> bool {
		matches!(self.status, ViewTransitionStatus::Unsupported)
	}

	/// Returns a failure message when the browser API failed.
	pub fn error(&self) -> Option<&str> {
		match &self.status {
			ViewTransitionStatus::Failed(message) => Some(message.as_str()),
			_ => None,
		}
	}

	/// Returns the underlying browser transition on WASM when one was started.
	#[cfg(wasm)]
	pub fn transition(&self) -> Option<&wasm_bindgen::JsValue> {
		self.transition.as_ref()
	}

	/// Skip the active browser transition on WASM.
	#[cfg(wasm)]
	pub fn skip_transition(&self) -> Result<(), String> {
		if let Some(transition) = &self.transition {
			use wasm_bindgen::{JsCast, JsValue};

			let skip = js_sys::Reflect::get(transition, &JsValue::from_str("skipTransition"))
				.map_err(|e| format!("Failed to read skipTransition: {:?}", e))?;

			if skip.is_function() {
				skip.unchecked_ref::<js_sys::Function>()
					.call0(transition)
					.map_err(|e| format!("Failed to skip view transition: {:?}", e))?;
			}
		}
		Ok(())
	}
}

/// Start a browser view transition around `update` when the API is available.
///
/// On native targets, and on browsers without `document.startViewTransition`,
/// the update closure still runs and the handle reports `Unsupported`.
#[cfg(native)]
pub fn start_view_transition<F>(update: F) -> ViewTransitionHandle
where
	F: FnOnce() + 'static,
{
	update();
	ViewTransitionHandle::from_status(ViewTransitionStatus::Unsupported)
}

/// Start a browser view transition around `update` when the API is available.
///
/// On native targets, and on browsers without `document.startViewTransition`,
/// the update closure still runs and the handle reports `Unsupported`.
#[cfg(wasm)]
pub fn start_view_transition<F>(update: F) -> ViewTransitionHandle
where
	F: FnOnce() + 'static,
{
	use std::cell::RefCell;
	use std::rc::Rc;

	use wasm_bindgen::{JsCast, JsValue, closure::Closure};

	let Some(window) = web_sys::window() else {
		update();
		return ViewTransitionHandle::from_status(ViewTransitionStatus::Failed(
			"Window object not available".to_string(),
		));
	};

	let Some(document) = window.document() else {
		update();
		return ViewTransitionHandle::from_status(ViewTransitionStatus::Failed(
			"Document object not available".to_string(),
		));
	};

	let start_fn =
		js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("startViewTransition"))
			.unwrap_or(JsValue::UNDEFINED);

	if !start_fn.is_function() {
		update();
		return ViewTransitionHandle::from_status(ViewTransitionStatus::Unsupported);
	}

	let update_cell = Rc::new(RefCell::new(Some(Box::new(update) as Box<dyn FnOnce()>)));
	let callback_value = Closure::once_into_js({
		let update_cell = Rc::clone(&update_cell);
		move || {
			if let Some(update) = update_cell.borrow_mut().take() {
				update();
			}
		}
	});

	let callback = callback_value.unchecked_ref::<js_sys::Function>();
	match start_fn
		.unchecked_ref::<js_sys::Function>()
		.call1(document.as_ref(), callback)
	{
		Ok(transition) => ViewTransitionHandle::started(transition),
		Err(error) => {
			if let Some(update) = update_cell.borrow_mut().take() {
				update();
			}
			ViewTransitionHandle::from_status(ViewTransitionStatus::Failed(format!(
				"document.startViewTransition failed: {:?}",
				error
			)))
		}
	}
}

/// SSR-safe wrapper that marks a subtree as a view-transition participant.
pub struct ViewTransitionBoundary {
	name: Option<Cow<'static, str>>,
	content_fn: Box<dyn Fn() -> Page>,
}

impl ViewTransitionBoundary {
	/// Create a boundary without a `view-transition-name`.
	pub fn new() -> Self {
		Self {
			name: None,
			content_fn: Box::new(Page::empty),
		}
	}

	/// Set the CSS `view-transition-name` for the boundary wrapper.
	pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
		self.name = Some(sanitize_view_transition_name(name.into()));
		self
	}

	/// Set the boundary content closure.
	pub fn content(mut self, f: impl Fn() -> Page + 'static) -> Self {
		self.content_fn = Box::new(f);
		self
	}

	/// Render the boundary.
	pub fn render(&self) -> Page {
		let mut element = PageElement::new("div").attr("data-rh-view-transition", "boundary");

		if let Some(name) = &self.name {
			element = element
				.attr("data-rh-view-transition-name", name.clone())
				.attr("style", format!("view-transition-name: {};", name));
		}

		element.child((self.content_fn)()).into_page()
	}
}

impl Default for ViewTransitionBoundary {
	fn default() -> Self {
		Self::new()
	}
}

impl IntoPage for ViewTransitionBoundary {
	fn into_page(self) -> Page {
		self.render()
	}
}

fn sanitize_view_transition_name(name: Cow<'static, str>) -> Cow<'static, str> {
	let value = name.as_ref();
	let mut sanitized = String::with_capacity(value.len().max(1) + 6);

	if value.is_empty() || needs_identifier_prefix(value) {
		sanitized.push_str("rh-vt-");
	}

	for ch in value.chars() {
		if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
			sanitized.push(ch);
		} else {
			sanitized.push('_');
		}
	}

	if sanitized.eq_ignore_ascii_case("inherit")
		|| sanitized.eq_ignore_ascii_case("initial")
		|| sanitized.eq_ignore_ascii_case("none")
		|| sanitized.eq_ignore_ascii_case("revert")
		|| sanitized.eq_ignore_ascii_case("revert-layer")
		|| sanitized.eq_ignore_ascii_case("unset")
	{
		sanitized.insert_str(0, "rh-vt-");
	}

	match name {
		Cow::Borrowed(value) if sanitized == value => Cow::Borrowed(value),
		_ => Cow::Owned(sanitized),
	}
}

fn needs_identifier_prefix(value: &str) -> bool {
	let mut chars = value.chars();
	!matches!(
		chars.next(),
		Some(ch) if ch.is_ascii_alphabetic() || ch == '_'
	)
}
