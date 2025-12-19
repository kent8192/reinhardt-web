//! Type-safe Attribute Builders
//!
//! This module provides type-safe helpers for working with HTML attributes,
//! especially boolean attributes and ARIA attributes.

use crate::builder::html::ElementBuilder;
use crate::dom::Element;

/// Extension trait for boolean attributes
///
/// Boolean attributes in HTML (like `disabled`, `checked`) are present or absent,
/// not true/false. This trait provides a type-safe API for managing them.
pub trait BooleanAttributes {
	/// Set the `disabled` attribute
	///
	/// ## Example
	///
	/// ```ignore
	/// button().disabled(true).build()  // <button disabled></button>
	/// button().disabled(false).build() // <button></button>
	/// ```
	fn disabled(self, value: bool) -> Self;

	/// Set the `checked` attribute (for checkboxes and radio buttons)
	///
	/// ## Example
	///
	/// ```ignore
	/// input().attr("type", "checkbox").checked(true).build()
	/// ```
	fn checked(self, value: bool) -> Self;

	/// Set the `selected` attribute (for `<option>` elements)
	///
	/// ## Example
	///
	/// ```ignore
	/// option().attr("value", "1").selected(true).build()
	/// ```
	fn selected(self, value: bool) -> Self;

	/// Set the `readonly` attribute (for input elements)
	///
	/// ## Example
	///
	/// ```ignore
	/// input().attr("type", "text").readonly(true).build()
	/// ```
	fn readonly(self, value: bool) -> Self;

	/// Set the `required` attribute (for form inputs)
	///
	/// ## Example
	///
	/// ```ignore
	/// input().attr("type", "text").required(true).build()
	/// ```
	fn required(self, value: bool) -> Self;

	/// Set the `autofocus` attribute
	///
	/// ## Example
	///
	/// ```ignore
	/// input().attr("type", "text").autofocus(true).build()
	/// ```
	fn autofocus(self, value: bool) -> Self;

	/// Set the `multiple` attribute (for `<select>` elements)
	///
	/// ## Example
	///
	/// ```ignore
	/// select().multiple(true).build()
	/// ```
	fn multiple(self, value: bool) -> Self;
}

impl BooleanAttributes for ElementBuilder {
	fn disabled(self, value: bool) -> Self {
		if value {
			self.attr("disabled", "")
		} else {
			// Note: Removing attributes requires direct Element access
			// For now, we just don't set it
			self
		}
	}

	fn checked(self, value: bool) -> Self {
		if value {
			self.attr("checked", "")
		} else {
			self
		}
	}

	fn selected(self, value: bool) -> Self {
		if value {
			self.attr("selected", "")
		} else {
			self
		}
	}

	fn readonly(self, value: bool) -> Self {
		if value {
			self.attr("readonly", "")
		} else {
			self
		}
	}

	fn required(self, value: bool) -> Self {
		if value {
			self.attr("required", "")
		} else {
			self
		}
	}

	fn autofocus(self, value: bool) -> Self {
		if value {
			self.attr("autofocus", "")
		} else {
			self
		}
	}

	fn multiple(self, value: bool) -> Self {
		if value {
			self.attr("multiple", "")
		} else {
			self
		}
	}
}

/// Extension trait for ARIA attributes (accessibility)
///
/// ARIA (Accessible Rich Internet Applications) attributes improve accessibility
/// for screen readers and assistive technologies.
pub trait AriaAttributes {
	/// Set the `aria-label` attribute
	///
	/// Provides an accessible label for screen readers.
	///
	/// ## Example
	///
	/// ```ignore
	/// button().aria_label("Close dialog").build()
	/// ```
	fn aria_label(self, label: &str) -> Self;

	/// Set the `aria-hidden` attribute
	///
	/// Hides the element from screen readers.
	///
	/// ## Example
	///
	/// ```ignore
	/// div().aria_hidden(true).build()
	/// ```
	fn aria_hidden(self, hidden: bool) -> Self;

	/// Set the `aria-expanded` attribute
	///
	/// Indicates whether a collapsible element is expanded or collapsed.
	///
	/// ## Example
	///
	/// ```ignore
	/// button().aria_expanded(false).build()
	/// ```
	fn aria_expanded(self, expanded: bool) -> Self;

	/// Set the `aria-live` attribute
	///
	/// Indicates that an element will be updated, and describes the types of updates
	/// the user agents, assistive technologies, and user can expect.
	///
	/// ## Example
	///
	/// ```ignore
	/// div().aria_live("polite").build()
	/// ```
	fn aria_live(self, value: &str) -> Self;

	/// Set the `aria-describedby` attribute
	///
	/// Identifies the element (or elements) that describes the object.
	///
	/// ## Example
	///
	/// ```ignore
	/// input().aria_describedby("error-message").build()
	/// ```
	fn aria_describedby(self, id: &str) -> Self;

	/// Set the `aria-labelledby` attribute
	///
	/// Identifies the element (or elements) that labels the current element.
	///
	/// ## Example
	///
	/// ```ignore
	/// input().aria_labelledby("label-id").build()
	/// ```
	fn aria_labelledby(self, id: &str) -> Self;

	/// Set the `role` attribute (ARIA role)
	///
	/// ## Example
	///
	/// ```ignore
	/// div().role("button").build()
	/// ```
	fn role(self, role: &str) -> Self;
}

impl AriaAttributes for ElementBuilder {
	fn aria_label(self, label: &str) -> Self {
		self.attr("aria-label", label)
	}

	fn aria_hidden(self, hidden: bool) -> Self {
		self.attr("aria-hidden", if hidden { "true" } else { "false" })
	}

	fn aria_expanded(self, expanded: bool) -> Self {
		self.attr("aria-expanded", if expanded { "true" } else { "false" })
	}

	fn aria_live(self, value: &str) -> Self {
		self.attr("aria-live", value)
	}

	fn aria_describedby(self, id: &str) -> Self {
		self.attr("aria-describedby", id)
	}

	fn aria_labelledby(self, id: &str) -> Self {
		self.attr("aria-labelledby", id)
	}

	fn role(self, role: &str) -> Self {
		self.attr("role", role)
	}
}

/// Helper for removing attributes from an Element
///
/// Since ElementBuilder uses a fluent API, attribute removal must be done
/// on the Element directly after building.
///
/// ## Example
///
/// ```ignore
/// let element = button().disabled(true).build();
/// remove_attribute(&element, "disabled");
/// ```
pub fn remove_attribute(element: &Element, name: &str) {
	let _ = element.remove_attribute(name);
}
