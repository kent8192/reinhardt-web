//! IntoPage trait and Page enum for component rendering.
//!
//! This module re-exports the core Page types from `reinhardt-types` and provides
//! WASM-specific extensions for DOM mounting.

// Re-export core types from reinhardt-types
pub use reinhardt_core::types::page::{
	Head, IntoPage, LinkTag, MetaTag, MountError, Outlet, Page, PageElement, PageEventHandler,
	Reactive, ReactiveIf, ScriptTag, StyleTag,
};

#[cfg(native)]
pub use reinhardt_core::types::page::NativeEvent;
// Re-export boolean attribute utilities (used in WASM mount)
// Note: EventType is re-exported from dom::event module
#[cfg(wasm)]
pub(super) use reinhardt_core::types::page::{BOOLEAN_ATTRS, is_boolean_attr_truthy};

#[cfg(wasm)]
use crate::component::reactive_if::{
	ReactiveIfNode, ReactiveNode, store_reactive_node, with_reactive_node_transaction,
};
#[cfg(wasm)]
use crate::dom::control_binding::ControlBindingController;
#[cfg(wasm)]
use crate::dom::{Element, EventHandle};

/// Extension trait for mounting Page to DOM (WASM only).
///
/// This trait provides the `mount()` method for `Page` which is only available
/// in WASM environments where actual DOM manipulation is possible.
#[cfg(wasm)]
pub trait PageExt {
	/// Mounts the view to a DOM element (client-side only).
	fn mount(self, parent: &Element) -> Result<(), MountError>;
}

#[cfg(wasm)]
impl PageExt for Page {
	fn mount(self, parent: &Element) -> Result<(), MountError> {
		mount_inner(self, parent)
	}
}

#[cfg(wasm)]
fn mount_inner(page: Page, parent: &Element) -> Result<(), MountError> {
	use crate::dom::document;

	match page {
		Page::Element(el) => {
			let doc = document();
			let (tag, attrs, children, _is_void, event_handlers, control_binding) =
				el.into_parts_with_control_binding();

			let element = doc
				.create_element(&tag)
				.map_err(|_| MountError::CreateElementFailed)?;

			for (name, value) in attrs {
				// Skip boolean attributes with falsy values (empty, "false", "0")
				// This ensures `disabled: ""` doesn't set the attribute
				let name_str: &str = name.as_ref();
				let value_str: &str = value.as_ref();
				let is_boolean = BOOLEAN_ATTRS.contains(&name_str);
				let is_falsy = !is_boolean_attr_truthy(value_str);

				if is_boolean && is_falsy {
					continue;
				}

				element
					.set_attribute(&name, &value)
					.map_err(|err_str: String| {
						// Log detailed error to browser console
						let msg: wasm_bindgen::JsValue = format!(
							"[SetAttributeFailed] attribute='{}', value='{}'",
							name, value
						)
						.into();
						let label: wasm_bindgen::JsValue = "Error message:".into();
						let err_msg: wasm_bindgen::JsValue = err_str.into();
						web_sys::console::error_3(&msg, &label, &err_msg);
						MountError::SetAttributeFailed
					})?;
			}

			let mount_children_before_binding = tag == "select";
			let mount_element = || {
				let mut children = children.into_iter();
				if mount_children_before_binding {
					for child in children.by_ref() {
						mount_inner(child, &element)?;
					}
				}

				let binding_controller = control_binding
					.map(|binding| ControlBindingController::mount(element.clone(), binding))
					.transpose()?;
				let mut event_handles: Vec<EventHandle> = Vec::new();

				for (event_type, handler) in event_handlers {
					let handler_clone = handler.clone();
					#[cfg(feature = "i18n")]
					let i18n_context = crate::i18n::current_i18n_callback_context();
					event_handles.push(element.add_event_listener_with_event(
						event_type.as_str(),
						move |event| {
							#[cfg(feature = "i18n")]
							{
								crate::i18n::with_optional_i18n_context(
									i18n_context.as_ref(),
									|| handler_clone(event),
								);
							}
							#[cfg(not(feature = "i18n"))]
							handler_clone(event);
						},
					));
				}

				for child in children {
					mount_inner(child, &element)?;
				}

				parent
					.append_child(element)
					.map_err(|_| MountError::AppendChildFailed)?;
				store_reactive_node((binding_controller, event_handles));
				Ok::<(), MountError>(())
			};
			with_reactive_node_transaction(mount_element)?;
		}
		Page::Text(text) => {
			let window = web_sys::window().ok_or(MountError::NoWindow)?;
			let document = window.document().ok_or(MountError::NoDocument)?;
			let text_node = document.create_text_node(&text);
			parent
				.inner()
				.append_child(&text_node)
				.map_err(|_| MountError::AppendChildFailed)?;
		}
		Page::Fragment(children) => {
			for child in children {
				mount_inner(child, parent)?;
			}
		}
		Page::KeyedFragment(children) => {
			for (_, child) in children {
				mount_inner(child, parent)?;
			}
		}
		Page::Outlet(outlet) => {
			let id = outlet.id().map(str::to_string);
			if let Some(child) = outlet.into_child() {
				mount_inner(child, parent)?;
			} else if let Some(id) = id {
				let doc = document();
				let host = doc
					.create_element("reinhardt-outlet")
					.map_err(|_| MountError::CreateElementFailed)?;
				host.set_attribute("data-rh-outlet-id", &id)
					.map_err(|_| MountError::SetAttributeFailed)?;
				host.set_attribute("style", "display: contents;")
					.map_err(|_| MountError::SetAttributeFailed)?;
				parent
					.append_child(host)
					.map_err(|_| MountError::AppendChildFailed)?;
			}
		}
		Page::Empty => {}
		Page::WithHead { view, .. } => {
			// On client-side, head is handled separately; just mount the content
			mount_inner(*view, parent)?;
		}
		Page::ReactiveIf(reactive_if) => {
			// Decompose the ReactiveIf to get the closures
			let (condition, then_view, else_view) = reactive_if.into_parts();

			// Create a ReactiveIfNode that manages DOM updates reactively.
			// The node uses an Effect to monitor condition changes and swaps
			// DOM nodes when the condition value changes.
			let node = ReactiveIfNode::new(parent, condition, then_view, else_view);
			// Store the node to keep it alive for the lifetime of the DOM element
			store_reactive_node(node);
		}
		Page::Reactive(reactive) => {
			// Get the render closure from the Reactive
			let render = reactive.into_render();

			// Create a ReactiveNode that manages DOM updates reactively.
			// The node uses an Effect to monitor dependency changes and
			// re-renders when they change.
			let node = ReactiveNode::new(parent, render);
			// Store the node to keep it alive for the lifetime of the DOM element
			store_reactive_node(node);
		}
		Page::Suspense(node) => {
			mount_inner(node.render_branch(), parent)?;
		}
		Page::Deferred(node) => {
			mount_inner(node.content(), parent)?;
		}
	}

	Ok(())
}

/// Extension trait for mounting Page (non-WASM stub).
///
/// In non-WASM environments, this trait provides a no-op stub for API compatibility.
#[cfg(native)]
pub trait PageExt {
	/// Mounts the view (non-WASM stub).
	///
	/// In non-WASM environments, this function is a no-op stub that always succeeds.
	/// The `parent` parameter is unused and exists only for API compatibility.
	fn mount<T>(self, _parent: &T) -> Result<(), MountError>;
}

#[cfg(native)]
impl PageExt for Page {
	fn mount<T>(self, _parent: &T) -> Result<(), MountError> {
		Ok(())
	}
}
