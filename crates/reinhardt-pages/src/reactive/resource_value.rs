//! Resource and Action composition helpers.
//!
//! These helpers let UI code read the latest successful mutation result before
//! falling back to the underlying resource state.

use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use reinhardt_core::reactive::deps::{Deps, Trackable};

use super::Effect;
use super::hooks::{Action, ActionPhase};
use super::resource::{Resource, ResourceState};
use crate::form_state::{FormAction, FormRuntimeSource};

/// UI-oriented state for a latest resource value read.
#[derive(Clone, Debug, PartialEq)]
pub enum LatestResourceState<T, E> {
	/// The underlying resource is loading and no action success is available.
	Loading,
	/// The latest success value matched the caller-provided empty predicate.
	Empty,
	/// A latest resource or action value is available.
	Success(T),
	/// The underlying resource failed and no action success is available.
	Error(E),
}

impl<T, E> LatestResourceState<T, E> {
	/// Returns `true` if the state is `Loading`.
	pub fn is_loading(&self) -> bool {
		matches!(self, LatestResourceState::Loading)
	}

	/// Returns `true` if the state is `Empty`.
	pub fn is_empty(&self) -> bool {
		matches!(self, LatestResourceState::Empty)
	}

	/// Returns `true` if the state is `Success`.
	pub fn is_success(&self) -> bool {
		matches!(self, LatestResourceState::Success(_))
	}

	/// Returns `true` if the state is `Error`.
	pub fn is_error(&self) -> bool {
		matches!(self, LatestResourceState::Error(_))
	}

	/// Returns the success value if available.
	pub fn as_ref(&self) -> Option<&T> {
		match self {
			LatestResourceState::Success(value) => Some(value),
			_ => None,
		}
	}

	/// Returns the error value if available.
	pub fn error(&self) -> Option<&E> {
		match self {
			LatestResourceState::Error(error) => Some(error),
			_ => None,
		}
	}
}

/// Builder returned by [`use_latest_resource_value`].
pub struct LatestResourceValueBuilder<T: Clone + 'static, E: Clone + 'static> {
	resource: Resource<T, E>,
	actions: Vec<Action<T, E>>,
	refetch_on_success: bool,
}

impl<T: Clone + 'static, E: Clone + 'static> LatestResourceValueBuilder<T, E> {
	/// Add an action whose success result can override the resource value.
	///
	/// Later actions have higher priority than earlier actions.
	pub fn with_action<A>(mut self, action: A) -> Self
	where
		A: Borrow<Action<T, E>>,
	{
		self.actions.push(*action.borrow());
		self
	}

	/// Adds a validated form action without exposing its dispatch handle.
	pub fn with_form_action<Form, FormDeps>(
		mut self,
		action: &FormAction<Form, FormDeps, T, E>,
	) -> Self
	where
		Form: FormRuntimeSource,
		FormDeps: Clone + PartialEq + 'static,
		E: Display,
	{
		self.actions.push(action.action());
		self
	}

	/// Refetch the underlying resource whenever a tracked action enters success.
	pub fn refetch_on_success(mut self) -> Self {
		self.refetch_on_success = true;
		self
	}

	/// Build the latest-value handle.
	pub fn build(self) -> LatestResourceValue<T, E> {
		LatestResourceValue::new(self.resource, self.actions, self.refetch_on_success)
	}
}

/// Compose a resource with one or more action success values.
pub fn use_latest_resource_value<T, E>(resource: Resource<T, E>) -> LatestResourceValueBuilder<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	LatestResourceValueBuilder {
		resource,
		actions: Vec::new(),
		refetch_on_success: false,
	}
}

/// Handle that reads an action success value before falling back to a resource.
///
/// Action priority is explicit: each later [`latest_after`](Self::latest_after)
/// or [`with_action`](LatestResourceValueBuilder::with_action) call wins over
/// earlier actions, and all action success values win over the resource value.
pub struct LatestResourceValue<T: Clone + 'static, E: Clone + 'static> {
	resource: Resource<T, E>,
	actions: Vec<Action<T, E>>,
	refetch_on_success: bool,
	_refetch_effect: Option<Rc<RefetchEffect>>,
}

struct RefetchEffect {
	effect: Effect,
}

impl Drop for RefetchEffect {
	fn drop(&mut self) {
		self.effect.dispose();
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for LatestResourceValue<T, E> {
	fn clone(&self) -> Self {
		Self {
			resource: self.resource,
			actions: self.actions.clone(),
			refetch_on_success: self.refetch_on_success,
			_refetch_effect: self._refetch_effect.clone(),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> LatestResourceValue<T, E> {
	fn new(resource: Resource<T, E>, actions: Vec<Action<T, E>>, refetch_on_success: bool) -> Self {
		let _refetch_effect = if refetch_on_success {
			build_refetch_effect(&resource, &actions)
		} else {
			None
		};

		Self {
			resource,
			actions,
			refetch_on_success,
			_refetch_effect,
		}
	}

	/// Add another action with higher priority than all previously added actions.
	pub fn latest_after<A>(mut self, action: A) -> Self
	where
		A: Borrow<Action<T, E>>,
	{
		self.actions.push(*action.borrow());
		self.rebuild_refetch_effect();
		self
	}

	/// Adds a validated form action with higher priority than prior actions.
	pub fn latest_after_form<Form, FormDeps>(
		mut self,
		action: &FormAction<Form, FormDeps, T, E>,
	) -> Self
	where
		Form: FormRuntimeSource,
		FormDeps: Clone + PartialEq + 'static,
		E: Display,
	{
		self.actions.push(action.action());
		self.rebuild_refetch_effect();
		self
	}

	/// Refetch the underlying resource whenever a tracked action enters success.
	pub fn refetch_on_success(mut self) -> Self {
		self.refetch_on_success = true;
		self.rebuild_refetch_effect();
		self
	}

	/// Returns the composed state, tracking resource and action dependencies.
	pub fn get(&self) -> ResourceState<T, E> {
		if let Some(value) = self.latest_action_success() {
			return ResourceState::Success(value);
		}

		self.resource.get()
	}

	/// Alias for [`get`](Self::get).
	pub fn state(&self) -> ResourceState<T, E> {
		self.get()
	}

	/// Returns the latest success value if resource or action data is available.
	pub fn value(&self) -> Option<T> {
		match self.get() {
			ResourceState::Success(value) => Some(value),
			ResourceState::Loading | ResourceState::Error(_) => None,
		}
	}

	/// Returns the underlying resource state without applying action overrides.
	pub fn resource_state(&self) -> ResourceState<T, E> {
		self.resource.get()
	}

	/// Returns the underlying resource error when no action success is available.
	pub fn error(&self) -> Option<E> {
		match self.get() {
			ResourceState::Error(error) => Some(error),
			ResourceState::Loading | ResourceState::Success(_) => None,
		}
	}

	/// Returns `true` if the composed state is loading.
	pub fn is_loading(&self) -> bool {
		self.get().is_loading()
	}

	/// Returns `true` if the composed state has a success value.
	pub fn is_success(&self) -> bool {
		self.get().is_success()
	}

	/// Returns `true` if the composed state is an error.
	pub fn is_error(&self) -> bool {
		self.get().is_error()
	}

	/// Classify a success value as empty with a caller-provided predicate.
	pub fn state_with_empty(&self, is_empty: impl FnOnce(&T) -> bool) -> LatestResourceState<T, E> {
		match self.get() {
			ResourceState::Loading => LatestResourceState::Loading,
			ResourceState::Error(error) => LatestResourceState::Error(error),
			ResourceState::Success(value) if is_empty(&value) => LatestResourceState::Empty,
			ResourceState::Success(value) => LatestResourceState::Success(value),
		}
	}

	/// Returns `true` if the latest success value matches the empty predicate.
	pub fn is_empty_by(&self, is_empty: impl FnOnce(&T) -> bool) -> bool {
		self.state_with_empty(is_empty).is_empty()
	}

	fn latest_action_success(&self) -> Option<T> {
		for action in self.actions.iter().rev() {
			if let ActionPhase::Success(value) = action.phase() {
				return Some(value);
			}
		}

		None
	}

	fn rebuild_refetch_effect(&mut self) {
		self._refetch_effect = if self.refetch_on_success {
			build_refetch_effect(&self.resource, &self.actions)
		} else {
			None
		};
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Resource<T, E> {
	/// Start reading action success values before this resource value.
	///
	/// The returned handle tracks reads from the action and the resource. Later
	/// [`LatestResourceValue::latest_after`] calls have higher priority.
	pub fn latest_after<A>(&self, action: A) -> LatestResourceValue<T, E>
	where
		A: Borrow<Action<T, E>>,
	{
		use_latest_resource_value(*self).with_action(action).build()
	}

	/// Composes this resource with a validated form action's successful result.
	pub fn latest_after_form<Form, FormDeps>(
		&self,
		action: &FormAction<Form, FormDeps, T, E>,
	) -> LatestResourceValue<T, E>
	where
		Form: FormRuntimeSource,
		FormDeps: Clone + PartialEq + 'static,
		E: Display,
	{
		use_latest_resource_value(*self)
			.with_form_action(action)
			.build()
	}
}

fn build_refetch_effect<T, E>(
	resource: &Resource<T, E>,
	actions: &[Action<T, E>],
) -> Option<Rc<RefetchEffect>>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	if actions.is_empty() {
		return None;
	}

	let deps = {
		let ids: Vec<_> = actions.iter().map(|action| action.node_id()).collect();
		Deps::from_signals(&ids)
	};
	let previous_success = Rc::new(RefCell::new(action_successes(actions)));
	let actions = actions.to_vec();
	let resource = *resource;

	Some(Rc::new(RefetchEffect {
		effect: Effect::new_with_deps(
			move || {
				let current_success = action_successes(&actions);
				let should_refetch = {
					let mut previous_success = previous_success.borrow_mut();
					let should_refetch = current_success
						.iter()
						.zip(previous_success.iter())
						.any(|(current, previous)| *current && !*previous);
					*previous_success = current_success;
					should_refetch
				};

				if should_refetch {
					resource.refetch();
				}

				None::<fn()>
			},
			deps,
		),
	}))
}

fn action_successes<T, E>(actions: &[Action<T, E>]) -> Vec<bool>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	actions
		.iter()
		.map(|action| matches!(action.phase(), ActionPhase::Success(_)))
		.collect()
}

#[cfg(test)]
mod tests {
	use serial_test::serial;

	use super::*;
	use crate::reactive::hooks::use_action;
	use crate::reactive::resource::use_resource;
	use crate::reactive::with_runtime;

	#[test]
	#[serial(reactive_runtime)]
	fn latest_after_prefers_later_action_success() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>("loaded".to_string()) },
				crate::deps![],
			);
			resource.set(ResourceState::Success("resource".to_string()));
			let refresh = use_action(|_: ()| async { Ok::<String, String>("refresh".to_string()) });
			let save = use_action(|_: ()| async { Ok::<String, String>("save".to_string()) });
			let latest = resource.latest_after(&refresh).latest_after(&save);

			assert_eq!(latest.get(), ResourceState::Success("resource".to_string()));

			refresh.force_success_for_test("refreshed".to_string());
			assert_eq!(
				latest.get(),
				ResourceState::Success("refreshed".to_string())
			);
			save.force_success_for_test("saved".to_string());
			assert_eq!(latest.get(), ResourceState::Success("saved".to_string()));

			save.reset();
			assert_eq!(
				latest.get(),
				ResourceState::Success("refreshed".to_string())
			);
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn action_error_does_not_replace_resource_error() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>("loaded".to_string()) },
				crate::deps![],
			);
			resource.set(ResourceState::Error("load failed".to_string()));
			let action =
				use_action(|_: ()| async { Err::<String, String>("save failed".to_string()) });
			let latest = resource.latest_after(&action);

			action.force_error_for_test("save failed".to_string());

			assert_eq!(
				latest.get(),
				ResourceState::Error("load failed".to_string())
			);
			assert_eq!(latest.error(), Some("load failed".to_string()));
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn state_with_empty_classifies_success_values() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<Vec<u32>, String>(Vec::new()) },
				crate::deps![],
			);
			resource.set(ResourceState::Success(Vec::new()));
			let action = use_action(|_: ()| async { Ok::<Vec<u32>, String>(vec![1, 2]) });
			let latest = resource.latest_after(&action);

			assert_eq!(
				latest.state_with_empty(Vec::is_empty),
				LatestResourceState::Empty
			);
			assert!(latest.is_empty_by(Vec::is_empty));

			action.force_success_for_test(vec![1, 2]);
			assert_eq!(
				latest.state_with_empty(Vec::is_empty),
				LatestResourceState::Success(vec![1, 2])
			);
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn refetch_on_success_refetches_resource_after_action_success() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>("loaded".to_string()) },
				crate::deps![],
			);
			resource.set(ResourceState::Success("resource".to_string()));
			let action = use_action(|_: ()| async { Ok::<String, String>("mutated".to_string()) });
			let latest = use_latest_resource_value(resource)
				.with_action(&action)
				.refetch_on_success()
				.build();

			assert_eq!(latest.get(), ResourceState::Success("resource".to_string()));

			action.force_success_for_test("mutated".to_string());

			assert_eq!(resource.get(), ResourceState::Loading);
			assert_eq!(latest.get(), ResourceState::Success("mutated".to_string()));
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn dropping_latest_resource_value_disposes_its_refetch_effect() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>("loaded".to_string()) },
				crate::deps![],
			);
			let action = use_action(|_: ()| async { Ok::<String, String>("saved".to_string()) });
			let latest = use_latest_resource_value(resource)
				.with_action(&action)
				.refetch_on_success()
				.build();
			let effect_id = latest
				._refetch_effect
				.as_ref()
				.expect("refetch-on-success should retain an effect")
				.effect
				.id();

			drop(latest);

			with_runtime(|runtime| {
				assert!(
					!runtime.has_node(effect_id),
					"dropping the latest-value handle must dispose its refetch effect"
				);
			});
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn rebuilding_latest_resource_value_disposes_replaced_refetch_effect() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>("loaded".to_string()) },
				crate::deps![],
			);
			let action = use_action(|_: ()| async { Ok::<String, String>("saved".to_string()) });
			let next_action =
				use_action(|_: ()| async { Ok::<String, String>("next".to_string()) });
			let latest = use_latest_resource_value(resource)
				.with_action(&action)
				.refetch_on_success()
				.build();
			let replaced_effect_id = latest
				._refetch_effect
				.as_ref()
				.expect("refetch-on-success should retain an effect")
				.effect
				.id();

			let latest = latest.latest_after(&next_action);

			with_runtime(|runtime| {
				assert!(
					!runtime.has_node(replaced_effect_id),
					"rebuilding the latest-value handle must dispose its replaced refetch effect"
				);
			});
			assert_ne!(
				latest
					._refetch_effect
					.as_ref()
					.expect("rebuilt latest value should retain an effect")
					.effect
					.id(),
				replaced_effect_id
			);
		});
	}
}
