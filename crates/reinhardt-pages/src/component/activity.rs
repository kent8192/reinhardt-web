//! State-preserved visibility boundary.
//!
//! `ActivityBoundary` keeps its child view in the rendered tree even when the
//! boundary is hidden. This differs from conditional rendering, which removes
//! the subtree and therefore cannot preserve DOM-owned state.

use crate::component::{IntoPage, Page, PageElement};

/// Visibility mode for an [`ActivityBoundary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityMode {
	/// Render the boundary as visible.
	#[default]
	Visible,
	/// Render the boundary as hidden while keeping children in the DOM tree.
	Hidden,
}

impl ActivityMode {
	/// Build a mode from a visible flag.
	pub const fn from_visible(visible: bool) -> Self {
		if visible { Self::Visible } else { Self::Hidden }
	}

	/// Returns `true` when the boundary should be visible.
	pub const fn is_visible(self) -> bool {
		matches!(self, Self::Visible)
	}

	/// Stable marker value used by rendered boundaries.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Visible => "visible",
			Self::Hidden => "hidden",
		}
	}
}

/// A boundary that hides UI without removing its subtree from the view tree.
pub struct ActivityBoundary {
	mode: ActivityMode,
	content_fn: Box<dyn Fn() -> Page>,
}

impl ActivityBoundary {
	/// Create a boundary with the given visibility mode.
	pub fn new(mode: ActivityMode) -> Self {
		Self {
			mode,
			content_fn: Box::new(Page::empty),
		}
	}

	/// Create a visible activity boundary.
	pub fn visible() -> Self {
		Self::new(ActivityMode::Visible)
	}

	/// Create a hidden activity boundary.
	pub fn hidden() -> Self {
		Self::new(ActivityMode::Hidden)
	}

	/// Set the boundary mode.
	pub fn mode(mut self, mode: ActivityMode) -> Self {
		self.mode = mode;
		self
	}

	/// Set the boundary visibility from a boolean.
	pub fn visible_when(self, visible: bool) -> Self {
		self.mode(ActivityMode::from_visible(visible))
	}

	/// Set the content closure.
	pub fn content(mut self, f: impl Fn() -> Page + 'static) -> Self {
		self.content_fn = Box::new(f);
		self
	}

	/// Return the configured visibility mode.
	pub const fn activity_mode(&self) -> ActivityMode {
		self.mode
	}

	/// Render the boundary.
	pub fn render(&self) -> Page {
		let content = (self.content_fn)();
		let mut element = PageElement::new("div")
			.attr("data-rh-activity", self.mode.as_str())
			.attr("data-rh-state-preserved", "true");

		if !self.mode.is_visible() {
			element = element.attr("hidden", "hidden").attr("aria-hidden", "true");
		}

		element.child(content).into_page()
	}
}

impl Default for ActivityBoundary {
	fn default() -> Self {
		Self::visible()
	}
}

impl IntoPage for ActivityBoundary {
	fn into_page(self) -> Page {
		self.render()
	}
}
