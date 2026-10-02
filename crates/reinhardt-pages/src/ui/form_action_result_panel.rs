use std::fmt::Display;
use std::rc::Rc;

use super::state::{ActionSlots, render_action_phase};
use crate::component::{Component, IntoPage, Page};
use crate::form_state::{FormAction, FormRuntimeSource};
use crate::reactive::ActionPhase;

type ValidationErrorSlot = Rc<dyn Fn(&str) -> Page>;

/// Renders validation and mutation phases from a validated form action.
pub struct FormActionResultPanel<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	action: FormAction<Form, Deps, T, E>,
	slots: ActionSlots<T, E>,
	validation_error: Option<ValidationErrorSlot>,
}

impl<Form, Deps, T, E> FormActionResultPanel<Form, Deps, T, E>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	T: Clone + 'static,
	E: Clone + Display + 'static,
{
	/// Creates a result panel backed by a validated form action.
	pub fn new(action: FormAction<Form, Deps, T, E>) -> Self {
		Self {
			action,
			slots: ActionSlots {
				idle: None,
				pending: None,
				success: None,
				error: None,
			},
			validation_error: None,
		}
	}

	/// Sets the idle view.
	pub fn idle<F: Fn() -> Page + 'static>(mut self, render: F) -> Self {
		self.slots.idle = Some(Rc::new(render));
		self
	}

	/// Sets the pending view.
	pub fn pending<F: Fn() -> Page + 'static>(mut self, render: F) -> Self {
		self.slots.pending = Some(Rc::new(render));
		self
	}

	/// Sets the successful mutation view.
	pub fn success<F: Fn(&T) -> Page + 'static>(mut self, render: F) -> Self {
		self.slots.success = Some(Rc::new(render));
		self
	}

	/// Sets the typed mutation-error view.
	pub fn error<F: Fn(&E) -> Page + 'static>(mut self, render: F) -> Self {
		self.slots.error = Some(Rc::new(render));
		self
	}

	/// Sets the generated-validation error view.
	pub fn validation_error<F: Fn(&str) -> Page + 'static>(mut self, render: F) -> Self {
		self.validation_error = Some(Rc::new(render));
		self
	}

	/// Renders the current form-action phase reactively.
	pub fn render(&self) -> Page {
		let action = self.action.clone();
		let slots = self.slots.clone();
		let validation_error = self.validation_error.clone();
		Page::reactive(move || {
			if action.is_pending() {
				return render_action_phase(ActionPhase::Pending, &slots);
			}
			match action.phase() {
				ActionPhase::Success(value) => {
					return render_action_phase(ActionPhase::Success(value), &slots);
				}
				ActionPhase::Error(error) => {
					return render_action_phase(ActionPhase::Error(error), &slots);
				}
				ActionPhase::Idle | ActionPhase::Pending => {}
			}
			if let Some(message) = action.error_message() {
				return validation_error
					.as_ref()
					.map_or_else(Page::empty, |slot| slot(&message));
			}
			render_action_phase(ActionPhase::Idle, &slots)
		})
	}
}

impl<Form, Deps, T, E> Component for FormActionResultPanel<Form, Deps, T, E>
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
		"FormActionResultPanel"
	}
}

impl<Form, Deps, T, E> IntoPage for FormActionResultPanel<Form, Deps, T, E>
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
