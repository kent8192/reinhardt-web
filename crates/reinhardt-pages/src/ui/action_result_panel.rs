use std::rc::Rc;

use super::state::{ActionSlots, render_action_phase};
use crate::component::{Component, IntoPage, Page};
use crate::reactive::Action;

/// Renders one of an action's idle, pending, success, or error views.
///
/// The panel is a state-driven view; it does not dispatch or mutate the
/// action. It is available from [`reinhardt_pages::ui`]:
///
/// ```rust,ignore
/// use reinhardt_pages::component::Page;
/// use reinhardt_pages::ui::ActionResultPanel;
///
/// let save_result = ActionResultPanel::new(save)
///     .pending(|| Page::text("Saving"))
///     .success(|value| Page::text(value.clone()))
///     .error(|error| Page::text(error.clone()));
/// ```
///
/// In this example `save` is an application-defined
/// [`Action`](crate::reactive::Action) whose success and error values are
/// `String`. The builder is generic over `T` and `E`; the slot signatures are
/// `Fn() -> Page` for `idle` and `pending`, `Fn(&T) -> Page` for `success`, and
/// `Fn(&E) -> Page` for `error`. An unset slot renders an empty page.
/// Error values are passed to the application-owned error slot without being
/// stringified, logged, or redacted automatically.
pub struct ActionResultPanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	action: Action<T, E>,
	slots: ActionSlots<T, E>,
}

impl<T, E> ActionResultPanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	/// Creates a result panel backed by `action`.
	///
	/// The panel initially selects the idle slot when one is configured.
	pub fn new(action: Action<T, E>) -> Self {
		Self {
			action,
			slots: ActionSlots {
				idle: None,
				pending: None,
				success: None,
				error: None,
			},
		}
	}

	/// Sets the view rendered while the action is idle.
	///
	/// The slot has the signature `Fn() -> Page` and is evaluated whenever the
	/// reactive panel renders the idle phase.
	pub fn idle<F>(mut self, render: F) -> Self
	where
		F: Fn() -> Page + 'static,
	{
		self.slots.idle = Some(Rc::new(render));
		self
	}

	/// Sets the view rendered while the action is pending.
	///
	/// The slot has the signature `Fn() -> Page`.
	pub fn pending<F>(mut self, render: F) -> Self
	where
		F: Fn() -> Page + 'static,
	{
		self.slots.pending = Some(Rc::new(render));
		self
	}

	/// Sets the view rendered after the action succeeds.
	///
	/// The slot has the signature `Fn(&T) -> Page`; the success value is borrowed
	/// for rendering and is not consumed by the panel.
	pub fn success<F>(mut self, render: F) -> Self
	where
		F: Fn(&T) -> Page + 'static,
	{
		self.slots.success = Some(Rc::new(render));
		self
	}

	/// Sets the view rendered after the action fails.
	///
	/// The slot has the signature `Fn(&E) -> Page`; applications decide whether
	/// and how much of the typed error should be shown to users.
	pub fn error<F>(mut self, render: F) -> Self
	where
		F: Fn(&E) -> Page + 'static,
	{
		self.slots.error = Some(Rc::new(render));
		self
	}

	/// Renders the slot for the action's current phase reactively.
	pub fn render(&self) -> Page {
		let action = self.action;
		let slots = self.slots.clone();
		Page::reactive(move || render_action_phase(action.phase(), &slots))
	}
}

impl<T, E> Component for ActionResultPanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	fn render(&self) -> Page {
		Self::render(self)
	}

	fn name() -> &'static str {
		"ActionResultPanel"
	}
}

impl<T, E> IntoPage for ActionResultPanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	fn into_page(self) -> Page {
		self.render()
	}
}

#[cfg(test)]
mod tests {
	use std::cell::Cell;
	use std::rc::Rc;

	use super::super::ActionResultPanel;
	use super::super::state::{ActionSlots, render_action_phase};
	use crate::component::{Component, IntoPage, Page};
	use crate::reactive::{ActionPhase, ReactiveScope, use_action};

	#[test]
	fn action_result_panel_renders_idle_success_and_error_slots() {
		ReactiveScope::run(|| {
			let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
			let panel = ActionResultPanel::new(action)
				.idle(|| Page::text("idle"))
				.pending(|| Page::text("pending"))
				.success(|value| Page::text(format!("success:{value}")))
				.error(|error| Page::text(format!("error:{error}")));
			let page = panel.render();

			assert_eq!(page.render_to_string(), "idle");

			action.force_success_for_test("saved".to_string());
			assert_eq!(page.render_to_string(), "success:saved");

			action.force_error_for_test("failed".to_string());
			assert_eq!(page.render_to_string(), "error:failed");
		});
	}

	#[test]
	fn action_result_panel_renders_pending_slot() {
		let slots = ActionSlots::<String, String> {
			pending: Some(Rc::new(|| Page::text("pending"))),
			..ActionSlots::default()
		};

		assert_eq!(
			render_action_phase(ActionPhase::Pending, &slots).render_to_string(),
			"pending"
		);
	}

	#[test]
	fn action_result_panel_uses_empty_page_for_missing_slots() {
		ReactiveScope::run(|| {
			let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
			let panel = ActionResultPanel::new(action);
			let page = panel.render();

			assert_eq!(page.render_to_string(), "");
			action.force_success_for_test("saved".to_string());
			assert_eq!(page.render_to_string(), "");
			action.force_error_for_test("failed".to_string());
			assert_eq!(page.render_to_string(), "");

			let pending_slots = ActionSlots::<String, String>::default();
			assert_eq!(
				render_action_phase(ActionPhase::Pending, &pending_slots).render_to_string(),
				""
			);
		});
	}

	#[test]
	fn action_result_panel_setters_replace_previous_slots() {
		ReactiveScope::run(|| {
			let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
			let panel = ActionResultPanel::new(action)
				.idle(|| Page::text("first-idle"))
				.idle(|| Page::text("second-idle"))
				.success(|value| Page::text(format!("first-success:{value}")))
				.success(|value| Page::text(format!("second-success:{value}")))
				.error(|error| Page::text(format!("first-error:{error}")))
				.error(|error| Page::text(format!("second-error:{error}")));

			assert_eq!(panel.render().render_to_string(), "second-idle");
			action.force_success_for_test("saved".to_string());
			assert_eq!(panel.render().render_to_string(), "second-success:saved");
			action.force_error_for_test("failed".to_string());
			assert_eq!(panel.render().render_to_string(), "second-error:failed");
		});
	}

	#[test]
	fn action_result_panel_render_does_not_dispatch_or_mutate_action() {
		ReactiveScope::run(|| {
			let dispatches = Rc::new(Cell::new(0));
			let dispatches_for_action = Rc::clone(&dispatches);
			let action = use_action(move |_: ()| {
				dispatches_for_action.set(dispatches_for_action.get() + 1);
				async { Ok::<String, String>("done".to_string()) }
			});
			let panel = ActionResultPanel::new(action).idle(|| Page::text("idle"));

			assert_eq!(panel.render().render_to_string(), "idle");
			assert_eq!(dispatches.get(), 0);
			assert_eq!(action.phase(), ActionPhase::Idle);
		});
	}

	#[test]
	fn action_result_panel_implements_component_and_into_page() {
		assert_eq!(
			ActionResultPanel::<String, String>::name(),
			"ActionResultPanel"
		);

		ReactiveScope::run(|| {
			let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
			let page = ActionResultPanel::new(action)
				.idle(|| Page::text("idle"))
				.into_page();

			assert_eq!(page.render_to_string(), "idle");
		});
	}
}
