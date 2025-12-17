//! Common reusable UI components
//!
//! This module contains common reusable components like confirmation modals.

use dominator::{Dom, clone, events, html};
use futures_signals::signal::{Mutable, SignalExt};
use parking_lot::Mutex;
use std::sync::Arc;

/// State for confirmation modal dialog
///
/// # Examples
///
/// ```ignore
/// use reinhardt_admin_ui::components::common::ConfirmModalState;
///
/// let modal = ConfirmModalState::new();
/// modal.show("Delete Item?", "This action cannot be undone.", || {
///     // Handle confirmation
/// });
/// ```
pub struct ConfirmModalState {
	/// Whether the modal is currently visible
	pub is_visible: Mutable<bool>,
	/// Modal title
	pub title: Mutable<String>,
	/// Modal message
	pub message: Mutable<String>,
	/// Callback to execute on confirmation
	on_confirm: Mutex<Option<Box<dyn Fn() + Send + Sync + 'static>>>,
}

impl ConfirmModalState {
	/// Create a new confirmation modal state
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			is_visible: Mutable::new(false),
			title: Mutable::new(String::new()),
			message: Mutable::new(String::new()),
			on_confirm: Mutex::new(None),
		})
	}

	/// Show the modal with the given title, message, and confirmation callback
	pub fn show<F>(&self, title: impl Into<String>, message: impl Into<String>, on_confirm: F)
	where
		F: Fn() + Send + Sync + 'static,
	{
		self.title.set(title.into());
		self.message.set(message.into());
		*self.on_confirm.lock() = Some(Box::new(on_confirm));
		self.is_visible.set(true);
	}

	/// Hide the modal
	pub fn hide(&self) {
		self.is_visible.set(false);
	}

	/// Execute the confirmation callback and hide the modal
	fn confirm(&self) {
		let callback = self.on_confirm.lock();
		if let Some(ref cb) = *callback {
			cb();
		}
		drop(callback);
		self.hide();
	}

	/// Cancel the modal (just hide it)
	fn cancel(&self) {
		self.hide();
	}
}

impl Default for ConfirmModalState {
	fn default() -> Self {
		Self {
			is_visible: Mutable::new(false),
			title: Mutable::new(String::new()),
			message: Mutable::new(String::new()),
			on_confirm: Mutex::new(None),
		}
	}
}

/// Render a confirmation modal dialog
///
/// The modal is rendered as an overlay with a dialog box containing:
/// - A title
/// - A message
/// - Cancel and Confirm buttons
///
/// # Arguments
///
/// * `state` - The modal state
///
/// # Examples
///
/// ```ignore
/// use reinhardt_admin_ui::components::common::{ConfirmModalState, render_confirm_modal};
///
/// let modal_state = ConfirmModalState::new();
/// let dom = render_confirm_modal(Arc::clone(&modal_state));
/// ```
pub fn render_confirm_modal(state: Arc<ConfirmModalState>) -> Dom {
	html!("div", {
		.class("modal-overlay")
		.visible_signal(state.is_visible.signal())
		.style_signal("display", state.is_visible.signal().map(|visible| {
			if visible { "flex" } else { "none" }
		}))
		// Inline CSS for modal overlay
		.style("position", "fixed")
		.style("top", "0")
		.style("left", "0")
		.style("right", "0")
		.style("bottom", "0")
		.style("background-color", "rgba(0, 0, 0, 0.5)")
		.style("justify-content", "center")
		.style("align-items", "center")
		.style("z-index", "1000")
		// Close on backdrop click
		.event(clone!(state => move |_: events::Click| {
			state.cancel();
		}))
		.child(html!("div", {
			.class("modal-dialog")
			// Prevent click from propagating to overlay
			.event(|event: events::Click| {
				event.stop_propagation();
			})
			// Inline CSS for modal dialog
			.style("background-color", "#ffffff")
			.style("border-radius", "8px")
			.style("box-shadow", "0 4px 20px rgba(0, 0, 0, 0.15)")
			.style("max-width", "400px")
			.style("width", "90%")
			.style("padding", "24px")
			.children(&mut [
				// Title
				html!("h3", {
					.class("modal-title")
					.style("margin", "0 0 16px 0")
					.style("font-size", "18px")
					.style("font-weight", "600")
					.style("color", "#333333")
					.text_signal(state.title.signal_cloned())
				}),
				// Message
				html!("p", {
					.class("modal-message")
					.style("margin", "0 0 24px 0")
					.style("font-size", "14px")
					.style("color", "#666666")
					.style("line-height", "1.5")
					.text_signal(state.message.signal_cloned())
				}),
				// Button container
				html!("div", {
					.class("modal-buttons")
					.style("display", "flex")
					.style("justify-content", "flex-end")
					.style("gap", "12px")
					.children(&mut [
						// Cancel button
						html!("button", {
							.class("btn btn-cancel")
							.style("padding", "8px 16px")
							.style("border", "1px solid #cccccc")
							.style("border-radius", "4px")
							.style("background-color", "#ffffff")
							.style("color", "#333333")
							.style("font-size", "14px")
							.style("cursor", "pointer")
							.text("Cancel")
							.event(clone!(state => move |_: events::Click| {
								state.cancel();
							}))
						}),
						// Confirm button
						html!("button", {
							.class("btn btn-confirm btn-danger")
							.style("padding", "8px 16px")
							.style("border", "none")
							.style("border-radius", "4px")
							.style("background-color", "#dc3545")
							.style("color", "#ffffff")
							.style("font-size", "14px")
							.style("cursor", "pointer")
							.text("Confirm")
							.event(clone!(state => move |_: events::Click| {
								state.confirm();
							}))
						}),
					])
				}),
			])
		}))
	})
}
