//! Public API compatibility tests for [`PageElement`] decomposition.

use reinhardt_core::reactive::{ReactiveScope, Signal};
use reinhardt_core::types::page::{ControlBinding, ControlKind, PageElement};

#[test]
fn into_parts_preserves_the_public_five_tuple() {
	let element = PageElement::new("button").attr("type", "button");

	let (tag, attrs, children, is_void, event_handlers) = element.into_parts();

	assert_eq!(tag, "button");
	assert_eq!(attrs, [("type".into(), "button".into())]);
	assert!(children.is_empty());
	assert!(!is_void);
	assert!(event_handlers.is_empty());
}

#[test]
fn into_parts_with_control_binding_returns_the_binding() {
	ReactiveScope::run(|| {
		let binding = ControlBinding::text(Signal::new("draft".to_owned()));
		let element = PageElement::new("input").control_binding(binding);

		let (_tag, _attrs, reactive_attrs, _children, _is_void, _event_handlers, binding) =
			element.into_parts_with_control_binding();

		assert!(reactive_attrs.is_empty());
		assert_eq!(binding.unwrap().kind(), ControlKind::Text);
	});
}
