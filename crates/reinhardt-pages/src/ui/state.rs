use std::rc::Rc;

use super::super::component::Page;
use super::super::reactive::{ActionPhase, ResourceState};

pub(crate) type PageSlot = Rc<dyn Fn() -> Page>;
pub(crate) type ValueSlot<T> = Rc<dyn Fn(&T) -> Page>;
pub(crate) type EmptyPredicate<T> = Rc<dyn Fn(&T) -> bool>;

#[derive(Clone, Default)]
pub(crate) struct ActionSlots<T, E> {
	pub(crate) idle: Option<PageSlot>,
	pub(crate) pending: Option<PageSlot>,
	pub(crate) success: Option<ValueSlot<T>>,
	pub(crate) error: Option<ValueSlot<E>>,
}

#[derive(Clone, Default)]
pub(crate) struct ResourceSlots<T, E> {
	pub(crate) loading: Option<PageSlot>,
	pub(crate) empty_if: Option<EmptyPredicate<T>>,
	pub(crate) empty: Option<ValueSlot<T>>,
	pub(crate) success: Option<ValueSlot<T>>,
	pub(crate) error: Option<ValueSlot<E>>,
}

pub(crate) fn render_action_phase<T: 'static, E: 'static>(
	phase: ActionPhase<T, E>,
	slots: &ActionSlots<T, E>,
) -> Page {
	match phase {
		ActionPhase::Idle => slots.idle.as_ref().map_or_else(Page::empty, |slot| slot()),
		ActionPhase::Pending => slots
			.pending
			.as_ref()
			.map_or_else(Page::empty, |slot| slot()),
		ActionPhase::Success(value) => slots
			.success
			.as_ref()
			.map_or_else(Page::empty, |slot| slot(&value)),
		ActionPhase::Error(error) => slots
			.error
			.as_ref()
			.map_or_else(Page::empty, |slot| slot(&error)),
	}
}

pub(crate) fn render_resource_state<T: 'static, E: 'static>(
	state: ResourceState<T, E>,
	slots: &ResourceSlots<T, E>,
) -> Page {
	match state {
		ResourceState::Loading => slots
			.loading
			.as_ref()
			.map_or_else(Page::empty, |slot| slot()),
		ResourceState::Success(value) => {
			if slots
				.empty_if
				.as_ref()
				.is_some_and(|predicate| predicate(&value))
			{
				return slots
					.empty
					.as_ref()
					.map_or_else(Page::empty, |slot| slot(&value));
			}
			slots
				.success
				.as_ref()
				.map_or_else(Page::empty, |slot| slot(&value))
		}
		ResourceState::Error(error) => slots
			.error
			.as_ref()
			.map_or_else(Page::empty, |slot| slot(&error)),
	}
}

#[cfg(test)]
mod tests {
	use std::rc::Rc;

	use super::{ActionSlots, ResourceSlots, render_action_phase, render_resource_state};
	use crate::component::Page;
	use crate::reactive::{ActionPhase, ResourceState};

	#[test]
	fn action_phase_renderer_selects_each_state_slot() {
		let slots = ActionSlots {
			idle: Some(Rc::new(|| Page::text("idle"))),
			pending: Some(Rc::new(|| Page::text("pending"))),
			success: Some(Rc::new(|value: &String| {
				Page::text(format!("success:{value}"))
			})),
			error: Some(Rc::new(|error: &String| {
				Page::text(format!("error:{error}"))
			})),
		};

		assert_eq!(
			render_action_phase(ActionPhase::Idle, &slots).render_to_string(),
			"idle"
		);
		assert_eq!(
			render_action_phase(ActionPhase::Pending, &slots).render_to_string(),
			"pending"
		);
		assert_eq!(
			render_action_phase(ActionPhase::Success("saved".to_string()), &slots)
				.render_to_string(),
			"success:saved"
		);
		assert_eq!(
			render_action_phase(ActionPhase::Error("failed".to_string()), &slots)
				.render_to_string(),
			"error:failed"
		);
	}

	#[test]
	fn resource_renderer_uses_empty_predicate_before_success_slot() {
		let slots = ResourceSlots {
			loading: Some(Rc::new(|| Page::text("loading"))),
			empty_if: Some(Rc::new(|items: &Vec<String>| items.is_empty())),
			empty: Some(Rc::new(|_| Page::text("empty"))),
			success: Some(Rc::new(|items: &Vec<String>| {
				Page::text(format!("success:{}", items[0]))
			})),
			error: Some(Rc::new(|error: &String| {
				Page::text(format!("error:{error}"))
			})),
		};

		assert_eq!(
			render_resource_state(ResourceState::Loading, &slots).render_to_string(),
			"loading"
		);
		assert_eq!(
			render_resource_state(ResourceState::Success(Vec::<String>::new()), &slots)
				.render_to_string(),
			"empty"
		);
		assert_eq!(
			render_resource_state(ResourceState::Success(vec!["one".to_string()]), &slots)
				.render_to_string(),
			"success:one"
		);
		assert_eq!(
			render_resource_state(ResourceState::Error("failed".to_string()), &slots)
				.render_to_string(),
			"error:failed"
		);
	}

	#[test]
	fn missing_slots_render_empty_pages() {
		let action_slots = ActionSlots::<String, String>::default();
		assert_eq!(
			render_action_phase(ActionPhase::Idle, &action_slots).render_to_string(),
			""
		);
		assert_eq!(
			render_action_phase(ActionPhase::Success("saved".to_string()), &action_slots)
				.render_to_string(),
			""
		);

		let resource_slots = ResourceSlots::<Vec<String>, String>::default();
		assert_eq!(
			render_resource_state(ResourceState::Loading, &resource_slots).render_to_string(),
			""
		);
		assert_eq!(
			render_resource_state(ResourceState::Success(Vec::new()), &resource_slots)
				.render_to_string(),
			""
		);
		assert_eq!(
			render_resource_state(ResourceState::Error("failed".to_string()), &resource_slots)
				.render_to_string(),
			""
		);
	}
}
