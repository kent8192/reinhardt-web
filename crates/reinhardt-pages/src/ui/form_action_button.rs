use std::fmt::Display;

use crate::component::{Component, IntoPage, Page, PageElement};
use crate::form_state::{FormAction, FormRuntimeSource};

/// Native submit button backed by a validated [`FormAction`].
///
/// Attach [`FormAction::submit_handler`] to the containing `form`. This button
/// deliberately has no click dispatcher, so all activation follows the form's
/// native submit lifecycle. The button bypasses browser constraint validation
/// so generated validation errors can be rendered by the form action.
pub struct FormActionButton<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	action: FormAction<Form, Deps, T, E>,
	children: Page,
	attrs: Vec<(String, String)>,
}

impl<Form, Deps, T, E> FormActionButton<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	/// Creates a native submit control for `action`.
	pub fn new<C>(action: FormAction<Form, Deps, T, E>, children: C) -> Self
	where
		C: IntoPage,
	{
		Self {
			action,
			children: children.into_page(),
			attrs: Vec::new(),
		}
	}

	/// Adds an attribute not managed by the component.
	pub fn attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.attrs.push((name.into(), value.into()));
		self
	}

	/// Renders the submit button.
	pub fn render(&self) -> Page {
		let mut button = PageElement::new("button")
			.attr("type", "submit")
			.attr("formnovalidate", "formnovalidate");
		for (name, value) in &self.attrs {
			if !name.eq_ignore_ascii_case("type")
				&& !name.eq_ignore_ascii_case("formnovalidate")
				&& !name.eq_ignore_ascii_case("disabled")
				&& !name.eq_ignore_ascii_case("aria-busy")
			{
				button = button.attr(name.clone(), value.clone());
			}
		}
		let disabled_action = self.action.clone();
		let busy_action = self.action.clone();
		button
			.reactive_attr("disabled", move || {
				disabled_action.is_pending().then(|| "disabled".into())
			})
			.reactive_attr("aria-busy", move || {
				busy_action.is_pending().then(|| "true".into())
			})
			.child(self.children.clone())
			.into_page()
	}
}

impl<Form, Deps, T, E> Component for FormActionButton<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	fn render(&self) -> Page {
		Self::render(self)
	}

	fn name() -> &'static str {
		"FormActionButton"
	}
}

impl<Form, Deps, T, E> IntoPage for FormActionButton<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	fn into_page(self) -> Page {
		self.render()
	}
}
