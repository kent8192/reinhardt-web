//! Event Reattachment for Hydration
//!
//! This module handles reattaching event listeners to DOM elements
//! during hydration. SSR cannot serialize JavaScript event handlers,
//! so they must be reattached on the client side.

#[cfg(any(wasm, test))]
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(wasm)]
use crate::dom::control_binding::ControlBindingController;
#[cfg(wasm)]
use crate::dom::{Element, EventHandle};
#[cfg(any(wasm, test))]
use reinhardt_core::types::page::EventName;
#[cfg(any(wasm, test))]
use reinhardt_core::types::page::event::event_spec;

/// A binding between an event and its handler.
#[derive(Debug, Clone)]
pub struct EventBinding {
	/// The event type.
	pub event_type: String,
	/// The hydration ID of the element.
	pub element_id: String,
	/// Whether the event should use capture phase.
	pub capture: bool,
	/// Whether the event should only fire once.
	pub once: bool,
	/// Whether to prevent default behavior.
	pub prevent_default: bool,
	/// Whether to stop propagation.
	pub stop_propagation: bool,
}

impl EventBinding {
	/// Creates a new event binding.
	pub fn new(event_type: impl Into<String>, element_id: impl Into<String>) -> Self {
		Self {
			event_type: event_type.into(),
			element_id: element_id.into(),
			capture: false,
			once: false,
			prevent_default: false,
			stop_propagation: false,
		}
	}

	/// Sets the capture option.
	pub fn capture(mut self, capture: bool) -> Self {
		self.capture = capture;
		self
	}

	/// Sets the once option.
	pub fn once(mut self, once: bool) -> Self {
		self.once = once;
		self
	}

	/// Sets the prevent_default option.
	pub fn prevent_default(mut self, prevent: bool) -> Self {
		self.prevent_default = prevent;
		self
	}

	/// Sets the stop_propagation option.
	pub fn stop_propagation(mut self, stop: bool) -> Self {
		self.stop_propagation = stop;
		self
	}
}

/// Registry for managing event handlers during hydration.
#[derive(Debug, Default)]
pub struct EventRegistry {
	/// Event handles indexed by element ID.
	#[cfg(wasm)]
	handles: HashMap<String, Vec<EventHandle>>,
	/// Event handles on elements without hydration IDs.
	#[cfg(wasm)]
	anonymous_handles: Vec<EventHandle>,
	/// Controlled form-element bindings installed during hydration.
	#[cfg(wasm)]
	control_bindings: Vec<ControlBindingController>,
	/// Whether hydration adopted a live control value into a signal.
	#[cfg(wasm)]
	control_binding_adopted: bool,
	/// Event handles for non-WASM (placeholder).
	#[cfg(native)]
	handles: HashMap<String, Vec<String>>,
}

impl EventRegistry {
	/// Creates a new event registry.
	pub fn new() -> Self {
		Self::default()
	}

	/// Registers an event handle for an element.
	#[cfg(wasm)]
	pub fn register(&mut self, element_id: impl Into<String>, handle: EventHandle) {
		self.handles
			.entry(element_id.into())
			.or_default()
			.push(handle);
	}

	/// Registers an event handle for an element without a hydration ID.
	#[cfg(wasm)]
	pub(crate) fn register_anonymous(&mut self, handle: EventHandle) {
		self.anonymous_handles.push(handle);
	}

	/// Retains a controlled form-element binding for the hydration lifetime.
	#[cfg(wasm)]
	pub(crate) fn register_control_binding(
		&mut self,
		controller: ControlBindingController,
		adopted: bool,
	) {
		self.control_binding_adopted |= adopted;
		self.control_bindings.push(controller);
	}

	/// Returns whether this registry adopted a live form-control value.
	#[cfg(wasm)]
	pub(crate) fn control_binding_adopted(&self) -> bool {
		self.control_binding_adopted
	}

	/// Propagates an adopted control value from a nested hydration branch.
	#[cfg(wasm)]
	pub(crate) fn mark_control_binding_adopted(&mut self) {
		self.control_binding_adopted = true;
	}

	/// Registers an event handle (non-WASM placeholder).
	#[cfg(native)]
	pub fn register(&mut self, element_id: impl Into<String>, handle: String) {
		self.handles
			.entry(element_id.into())
			.or_default()
			.push(handle);
	}

	/// Removes all event handles for an element.
	pub fn unregister(&mut self, element_id: &str) {
		self.handles.remove(element_id);
	}

	/// Removes all registered event handles.
	pub fn clear(&mut self) {
		self.handles.clear();
		#[cfg(wasm)]
		{
			self.anonymous_handles.clear();
			self.control_bindings.clear();
		}
	}

	/// Returns the number of registered keyed elements and anonymous handles.
	pub fn len(&self) -> usize {
		self.handles.len() + {
			#[cfg(wasm)]
			{
				self.anonymous_handles.len()
			}
			#[cfg(native)]
			{
				0
			}
		}
	}

	/// Returns true if no event handles are registered.
	pub fn is_empty(&self) -> bool {
		self.handles.is_empty() && {
			#[cfg(wasm)]
			{
				self.anonymous_handles.is_empty() && self.control_bindings.is_empty()
			}
			#[cfg(native)]
			{
				true
			}
		}
	}
}

/// Error type for event attachment.
#[derive(Debug, Clone)]
pub struct EventAttachError {
	/// The event type that failed.
	pub event_type: String,
	/// The reason for failure.
	pub reason: String,
}

impl std::fmt::Display for EventAttachError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Failed to attach '{}' event: {}",
			self.event_type, self.reason
		)
	}
}

impl std::error::Error for EventAttachError {}

/// Type alias for event handler functions.
#[cfg(wasm)]
pub(super) type EventHandler = Arc<dyn Fn(web_sys::Event) + 'static>;

/// Type alias for event handler functions (non-WASM placeholder).
#[cfg(native)]
pub(super) type EventHandler = Arc<dyn Fn() + Send + Sync + 'static>;

/// Options for attaching events (Phase 2-B).
///
/// Controls how events are attached during hydration, enabling
/// selective hydration for Island Architecture.
#[derive(Debug, Clone, Default)]
pub struct AttachOptions {
	/// If true, only attach events to islands (interactive components).
	/// Static content and full-hydration components are skipped.
	pub island_only: bool,

	/// If true, skip elements marked with `data-rh-static="true"`.
	/// This is useful for preserving server-rendered static content.
	pub skip_static: bool,
}

impl AttachOptions {
	/// Creates options for island-only attachment.
	pub fn island_only() -> Self {
		Self {
			island_only: true,
			skip_static: true,
		}
	}

	/// Creates options for full hydration (default).
	pub fn full_hydration() -> Self {
		Self::default()
	}
}

/// Attaches an event handler to a DOM element.
#[cfg(wasm)]
pub fn attach_event(
	element: &Element,
	event_type: &EventName,
	handler: EventHandler,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	let scope = reinhardt_core::reactive::scope::current_scope_id();
	#[cfg(feature = "i18n")]
	let i18n_context = crate::i18n::current_i18n_callback_context();
	let handle = element.add_event_listener_with_event(event_type.as_str(), move |event| {
		#[cfg(feature = "i18n")]
		{
			crate::i18n::with_optional_i18n_context(i18n_context.as_ref(), || {
				crate::callback::run_event_handler_in_scope(scope, &handler, event);
			});
		}
		#[cfg(not(feature = "i18n"))]
		crate::callback::run_event_handler_in_scope(scope, &handler, event);
	});

	// Get element ID for registry
	if let Some(id) = element.get_attribute("data-rh-id") {
		registry.register(id, handle);
	} else {
		registry.register_anonymous(handle);
	}

	Ok(())
}

/// Recursively attaches event handlers to a DOM subtree (Phase 2-B).
///
/// This function traverses the DOM tree starting from the given element
/// and attaches event handlers based on the provided options. It supports
/// selective hydration through the Island Architecture pattern.
///
/// # Arguments
///
/// * `element` - The root element to start traversal from
/// * `bindings` - Event bindings that specify which events to attach
/// * `handlers` - Event handler functions
/// * `options` - Options controlling which elements to hydrate
/// * `registry` - Event registry for tracking attached events
///
/// # Returns
///
/// `Ok(())` on success, or an `EventAttachError` if attachment fails.
///
/// # Behavior
///
/// - If `options.island_only` is true, only elements with `data-rh-island="true"` are hydrated
/// - If `options.skip_static` is true, elements with `data-rh-static="true"` are skipped
/// - Recursively processes child elements
#[cfg(wasm)]
pub fn attach_events_recursive(
	element: &Element,
	bindings: &[EventBinding],
	handlers: &HashMap<String, EventHandler>,
	options: &AttachOptions,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	// Check if this element should be skipped
	let should_skip = if options.skip_static {
		element.get_attribute("data-rh-static").as_deref() == Some("true")
	} else {
		false
	};

	if should_skip {
		return Ok(());
	}

	// Check if this is an island element
	let is_island = element.get_attribute("data-rh-island").as_deref() == Some("true");

	// Determine if we should attach events to this element
	let should_attach = if options.island_only {
		// In island-only mode, only attach to island elements
		is_island
	} else {
		// In full hydration mode, attach to all non-static elements
		true
	};

	// Attach events to this element if applicable
	if should_attach {
		// Find event bindings for this element by checking data-rh-id
		if let Some(element_id) = element.get_attribute("data-rh-id") {
			for binding in bindings {
				if binding.element_id == element_id
					&& let Some(handler) = handlers.get(&binding.event_type)
				{
					let event_type = event_name_from_string(&binding.event_type);
					attach_event(element, &event_type, handler.clone(), registry)?;
				}
			}
		}
	}

	// Recursively process children, unless this is an island boundary
	// (islands manage their own hydration)
	let should_recurse = if options.island_only && is_island {
		// If we're in island-only mode and this is an island,
		// don't recurse into children (they belong to this island's internal hydration)
		false
	} else {
		// Otherwise, recurse into children
		true
	};

	if should_recurse {
		let children = element.children();
		for child in &children {
			attach_events_recursive(child, bindings, handlers, options, registry)?;
		}
	}

	Ok(())
}

/// Non-WASM version for testing.
#[cfg(native)]
pub fn attach_event(
	_element: &str,
	event_type: &str,
	_handler: EventHandler,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	registry.register("test", event_type.to_string());
	Ok(())
}

/// Non-WASM version for testing (Phase 2-B).
#[cfg(native)]
// Allow dead_code: non-WASM stub for event attachment
#[allow(dead_code)]
pub fn attach_events_recursive(
	_element: &str,
	_bindings: &[EventBinding],
	_handlers: &HashMap<String, EventHandler>,
	_options: &AttachOptions,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	// Non-WASM stub: just register a dummy event
	registry.register("test-recursive", "recursive-event".to_string());
	Ok(())
}

/// Attaches multiple events based on bindings.
#[cfg(wasm)]
// Allow dead_code: WASM event attachment reserved for future hydration
#[allow(dead_code)]
pub(super) fn attach_events(
	element: &Element,
	bindings: &[EventBinding],
	handlers: &HashMap<String, EventHandler>,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	for binding in bindings {
		if let Some(handler) = handlers.get(&binding.event_type) {
			let event_type = event_name_from_string(&binding.event_type);
			attach_event(element, &event_type, handler.clone(), registry)?;
		}
	}
	Ok(())
}

/// Classifies a hydration event name through the authoritative catalog.
#[cfg(any(wasm, test))]
fn event_name_from_string(name: &str) -> EventName {
	match event_spec(name) {
		Some(spec) => EventName::Known(spec.kind),
		None => EventName::Custom(Cow::Owned(name.to_owned())),
	}
}

/// Non-WASM version for testing.
#[cfg(native)]
// Allow dead_code: non-WASM stub for event attachment
#[allow(dead_code)]
pub(super) fn attach_events(
	_element: &str,
	bindings: &[EventBinding],
	_handlers: &HashMap<String, EventHandler>,
	registry: &mut EventRegistry,
) -> Result<(), EventAttachError> {
	for binding in bindings {
		registry.register(&binding.element_id, binding.event_type.clone());
	}
	Ok(())
}

/// Detaches all event handlers from the registry.
// Allow dead_code: cleanup function reserved for future hydration lifecycle management
#[allow(dead_code)]
pub(super) fn detach_all(registry: &mut EventRegistry) {
	registry.clear();
}

// Phase 2-B Tests: Selective Event Attachment

#[test]
fn test_attach_options_default() {
	let options = AttachOptions::default();
	assert!(!options.island_only);
	assert!(!options.skip_static);
}

#[test]
fn test_attach_options_island_only() {
	let options = AttachOptions::island_only();
	assert!(options.island_only);
	assert!(options.skip_static);
}

#[test]
fn test_attach_options_full_hydration() {
	let options = AttachOptions::full_hydration();
	assert!(!options.island_only);
	assert!(!options.skip_static);
}

#[cfg(native)]
#[test]
fn test_attach_events_recursive_non_wasm() {
	let mut registry = EventRegistry::new();
	let bindings = vec![EventBinding::new("click", "el-1")];
	let handlers: HashMap<String, EventHandler> = HashMap::new();
	let options = AttachOptions::default();

	let result = attach_events_recursive("root", &bindings, &handlers, &options, &mut registry);

	assert!(result.is_ok());
	assert!(!registry.is_empty());
}
#[cfg(all(test, native))]
mod tests {
	use super::*;
	use reinhardt_core::types::page::{EventName, EventType};

	#[test]
	fn event_name_classification_preserves_known_and_custom_names() {
		assert_eq!(
			event_name_from_string("click"),
			EventName::Known(EventType::Click)
		);
		assert_eq!(
			event_name_from_string("editor:commit").as_str(),
			"editor:commit"
		);
		assert!(matches!(
			event_name_from_string("editor:commit"),
			EventName::Custom(_)
		));
	}

	#[test]
	fn test_event_binding_new() {
		let binding = EventBinding::new("click", "rh-0");
		assert_eq!(binding.event_type, "click");
		assert_eq!(binding.element_id, "rh-0");
		assert!(!binding.capture);
		assert!(!binding.once);
	}

	#[test]
	fn test_event_binding_builder() {
		let binding = EventBinding::new("submit", "rh-1")
			.capture(true)
			.once(true)
			.prevent_default(true)
			.stop_propagation(true);

		assert!(binding.capture);
		assert!(binding.once);
		assert!(binding.prevent_default);
		assert!(binding.stop_propagation);
	}

	#[test]
	fn test_event_registry_new() {
		let registry = EventRegistry::new();
		assert!(registry.is_empty());
		assert_eq!(registry.len(), 0);
	}

	#[test]
	fn test_event_registry_register() {
		let mut registry = EventRegistry::new();
		registry.register("el-1", "click".to_string());
		assert_eq!(registry.len(), 1);
		assert!(!registry.is_empty());
	}

	#[test]
	fn test_event_registry_unregister() {
		let mut registry = EventRegistry::new();
		registry.register("el-1", "click".to_string());
		registry.register("el-2", "submit".to_string());
		assert_eq!(registry.len(), 2);

		registry.unregister("el-1");
		assert_eq!(registry.len(), 1);
	}

	#[test]
	fn test_event_registry_clear() {
		let mut registry = EventRegistry::new();
		registry.register("el-1", "click".to_string());
		registry.register("el-2", "submit".to_string());

		registry.clear();
		assert!(registry.is_empty());
	}

	#[test]
	fn test_event_attach_error_display() {
		let err = EventAttachError {
			event_type: "click".to_string(),
			reason: "element not found".to_string(),
		};
		assert!(err.to_string().contains("click"));
		assert!(err.to_string().contains("element not found"));
	}

	#[test]
	fn test_attach_event_non_wasm() {
		let mut registry = EventRegistry::new();
		let handler: EventHandler = Arc::new(|| {});
		let result = attach_event("element", "click", handler, &mut registry);
		assert!(result.is_ok());
		assert!(!registry.is_empty());
	}

	#[test]
	fn test_attach_events_non_wasm() {
		let mut registry = EventRegistry::new();
		let bindings = vec![
			EventBinding::new("click", "el-1"),
			EventBinding::new("submit", "el-2"),
		];
		let handlers: HashMap<String, EventHandler> = HashMap::new();

		let result = attach_events("element", &bindings, &handlers, &mut registry);
		assert!(result.is_ok());
	}

	#[test]
	fn test_detach_all() {
		let mut registry = EventRegistry::new();
		registry.register("el-1", "click".to_string());
		registry.register("el-2", "submit".to_string());

		detach_all(&mut registry);
		assert!(registry.is_empty());
	}
}
