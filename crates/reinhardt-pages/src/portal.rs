//! Explicit portal mounting for Reinhardt Pages.
//!
//! Portals mount a [`Page`] into an existing DOM target outside the caller's
//! normal root. They are intentionally explicit: SSR renders only an optional
//! marker, and WASM mounting returns a [`PortalHandle`] that removes the portal
//! host when dropped.

use std::borrow::Cow;

use crate::component::{IntoPage, MountError, Page, PageElement};

#[cfg(wasm)]
use crate::component::PageExt;
#[cfg(wasm)]
use crate::component::reactive_if::{
	ReactiveNodeStore, clear_reactive_node_store, new_reactive_node_store, with_reactive_node_store,
};
#[cfg(wasm)]
use crate::dom::Element;

/// A DOM target that can receive a portal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalTarget {
	/// The document body.
	Body,
	/// An element selected by id.
	ElementId(Cow<'static, str>),
	/// An element selected by CSS selector.
	Selector(Cow<'static, str>),
}

impl PortalTarget {
	/// Targets the document body.
	pub const fn body() -> Self {
		Self::Body
	}

	/// Targets an element by id.
	pub fn element_id(id: impl Into<Cow<'static, str>>) -> Self {
		Self::ElementId(id.into())
	}

	/// Targets an element by CSS selector.
	pub fn selector(selector: impl Into<Cow<'static, str>>) -> Self {
		Self::Selector(selector.into())
	}

	/// Returns the deterministic marker value used for SSR placeholders.
	pub fn marker(&self) -> Cow<'_, str> {
		match self {
			Self::Body => Cow::Borrowed("body"),
			Self::ElementId(id) => Cow::Owned(format!("#{id}")),
			Self::Selector(selector) => Cow::Borrowed(selector.as_ref()),
		}
	}
}

/// A portal declaration.
#[derive(Debug, Clone)]
pub struct Portal {
	target: PortalTarget,
	view: Page,
}

impl Portal {
	/// Creates a portal for the given target and view.
	pub fn new(target: PortalTarget, view: impl IntoPage) -> Self {
		Self {
			target,
			view: view.into_page(),
		}
	}

	/// Creates a portal targeting an element by id.
	pub fn element_id(id: impl Into<Cow<'static, str>>, view: impl IntoPage) -> Self {
		Self::new(PortalTarget::element_id(id), view)
	}

	/// Creates a portal targeting a CSS selector.
	pub fn selector(selector: impl Into<Cow<'static, str>>, view: impl IntoPage) -> Self {
		Self::new(PortalTarget::selector(selector), view)
	}

	/// Returns the portal target.
	pub fn target(&self) -> &PortalTarget {
		&self.target
	}

	/// Renders the source-tree SSR placeholder for this portal.
	///
	/// Portal children are not duplicated into SSR output because the target DOM
	/// node is outside the source view. The marker makes the contract explicit
	/// while keeping hydration deterministic.
	pub fn placeholder(&self) -> Page {
		PageElement::new("template")
			.attr("data-rh-portal", self.target.marker().into_owned())
			.into_page()
	}

	/// Mounts this portal into its target.
	#[cfg(wasm)]
	pub fn mount(self) -> Result<PortalHandle, PortalError> {
		let window = web_sys::window().ok_or(PortalError::NoWindow)?;
		let document = window.document().ok_or(PortalError::NoDocument)?;
		let target = resolve_target(&document, &self.target)?;
		let host = document
			.create_element("div")
			.map_err(|_| PortalError::CreateHostFailed)?;

		host.set_attribute("data-rh-portal-host", &self.target.marker())
			.map_err(|_| PortalError::SetHostAttributeFailed)?;
		target
			.append_child(&host)
			.map_err(|_| PortalError::AppendHostFailed)?;

		let reactive_nodes = new_reactive_node_store();
		let wrapper = Element::new(host.clone());
		if let Err(error) = with_reactive_node_store(&reactive_nodes, || self.view.mount(&wrapper))
		{
			clear_reactive_node_store(&reactive_nodes);
			host.remove();
			return Err(PortalError::Mount(error));
		}

		Ok(PortalHandle::active(host, reactive_nodes))
	}

	/// Host-side portal mount stub.
	///
	/// Native rendering has no DOM target, so mounting is a no-op. Use
	/// [`Portal::placeholder`] when an SSR source-tree marker is needed.
	#[cfg(native)]
	pub fn mount(self) -> Result<PortalHandle, PortalError> {
		let Self { view, .. } = self;
		drop(view);
		Ok(PortalHandle::noop())
	}
}

/// Mounts a view into a portal target.
pub fn mount_portal(
	target: PortalTarget,
	view: impl IntoPage,
) -> Result<PortalHandle, PortalError> {
	Portal::new(target, view).mount()
}

#[cfg(wasm)]
fn resolve_target(
	document: &web_sys::Document,
	target: &PortalTarget,
) -> Result<web_sys::Element, PortalError> {
	match target {
		PortalTarget::Body => document
			.body()
			.map(Into::into)
			.ok_or(PortalError::TargetNotFound {
				target: target.marker().into_owned(),
			}),
		PortalTarget::ElementId(id) => {
			document
				.get_element_by_id(id)
				.ok_or_else(|| PortalError::TargetNotFound {
					target: target.marker().into_owned(),
				})
		}
		PortalTarget::Selector(selector) => document
			.query_selector(selector)
			.map_err(|error| PortalError::InvalidSelector {
				selector: selector.to_string(),
				message: format!("{error:?}"),
			})?
			.ok_or_else(|| PortalError::TargetNotFound {
				target: selector.to_string(),
			}),
	}
}

/// Error returned when a portal cannot be mounted.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalError {
	/// Window object not available.
	NoWindow,
	/// Document object not available.
	NoDocument,
	/// Portal target could not be found.
	TargetNotFound {
		/// The target marker or selector.
		target: String,
	},
	/// CSS selector target was invalid.
	InvalidSelector {
		/// The selector that failed.
		selector: String,
		/// Browser-provided error message.
		message: String,
	},
	/// Failed to create the portal host element.
	CreateHostFailed,
	/// Failed to set an attribute on the portal host.
	SetHostAttributeFailed,
	/// Failed to append the portal host to the target.
	AppendHostFailed,
	/// Failed to mount the portal view into the host.
	Mount(MountError),
}

impl std::fmt::Display for PortalError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoWindow => write!(f, "Window object not available"),
			Self::NoDocument => write!(f, "Document object not available"),
			Self::TargetNotFound { target } => {
				write!(f, "Portal target not found: {target}")
			}
			Self::InvalidSelector { selector, message } => {
				write!(f, "Invalid portal selector '{selector}': {message}")
			}
			Self::CreateHostFailed => write!(f, "Failed to create portal host"),
			Self::SetHostAttributeFailed => write!(f, "Failed to set portal host attribute"),
			Self::AppendHostFailed => write!(f, "Failed to append portal host"),
			Self::Mount(error) => write!(f, "Failed to mount portal view: {error}"),
		}
	}
}

impl std::error::Error for PortalError {}

impl From<MountError> for PortalError {
	fn from(error: MountError) -> Self {
		Self::Mount(error)
	}
}

/// RAII handle for a mounted portal.
pub struct PortalHandle {
	active: bool,
	#[cfg(wasm)]
	host: Option<web_sys::Element>,
	#[cfg(wasm)]
	reactive_nodes: Option<ReactiveNodeStore>,
}

impl PortalHandle {
	#[cfg(wasm)]
	fn active(host: web_sys::Element, reactive_nodes: ReactiveNodeStore) -> Self {
		Self {
			active: true,
			host: Some(host),
			reactive_nodes: Some(reactive_nodes),
		}
	}

	/// Creates a no-op handle for native targets.
	pub const fn noop() -> Self {
		Self {
			active: false,
			#[cfg(wasm)]
			host: None,
			#[cfg(wasm)]
			reactive_nodes: None,
		}
	}

	/// Returns true when this handle owns a live portal host.
	pub const fn is_active(&self) -> bool {
		self.active
	}

	/// Removes the portal host immediately.
	pub fn unmount(mut self) {
		self.detach();
	}

	fn detach(&mut self) {
		#[cfg(wasm)]
		if let Some(reactive_nodes) = self.reactive_nodes.take() {
			clear_reactive_node_store(&reactive_nodes);
		}

		#[cfg(wasm)]
		if let Some(host) = self.host.take() {
			host.remove();
		}

		self.active = false;
	}
}

impl std::fmt::Debug for PortalHandle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PortalHandle")
			.field("active", &self.active)
			.finish()
	}
}

impl Drop for PortalHandle {
	fn drop(&mut self) {
		self.detach();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn portal_target_marker_is_deterministic() {
		assert_eq!(PortalTarget::body().marker(), "body");
		assert_eq!(
			PortalTarget::element_id("modal-root").marker(),
			"#modal-root"
		);
		assert_eq!(
			PortalTarget::selector("[data-dialog-root]").marker(),
			"[data-dialog-root]"
		);
	}

	#[test]
	fn portal_placeholder_renders_source_tree_marker() {
		let portal = Portal::element_id("modal-root", Page::text("Dialog"));

		assert_eq!(
			portal.placeholder().render_to_string(),
			"<template data-rh-portal=\"#modal-root\"></template>"
		);
	}

	#[test]
	fn native_mount_returns_inactive_handle() {
		let handle = mount_portal(PortalTarget::body(), Page::text("Dialog")).unwrap();

		assert!(!handle.is_active());
	}
}
