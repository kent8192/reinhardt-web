use std::rc::Rc;

use super::state::{ResourceSlots, render_resource_state};
use crate::component::{Component, IntoPage, Page};
use crate::reactive::{LatestResourceValue, Resource, ResourceState};

#[derive(Clone)]
enum ResourcePanelSource<T: Clone + 'static, E: Clone + 'static> {
	Resource(Resource<T, E>),
	Latest(LatestResourceValue<T, E>),
}

impl<T: Clone + 'static, E: Clone + 'static> ResourcePanelSource<T, E> {
	fn get(&self) -> ResourceState<T, E> {
		match self {
			Self::Resource(resource) => resource.get(),
			Self::Latest(latest) => latest.get(),
		}
	}
}

/// Renders one of a resource's loading, empty, success, or error views.
///
/// `ResourcePanel` is available from [`reinhardt_pages::ui`] and can read a
/// [`Resource`] directly or a [`LatestResourceValue`] composed with action
/// results:
///
/// ```rust,ignore
/// use reinhardt_pages::component::Page;
/// use reinhardt_pages::ui::ResourcePanel;
///
/// let project_view = ResourcePanel::new(project)
///     .loading(|| Page::text("Loading"))
///     .empty_if(|projects: &Vec<String>| projects.is_empty())
///     .empty(|_| Page::text("No projects"))
///     .success(|projects| Page::text(projects.join(", ")))
///     .error(|error| Page::text(error.clone()));
/// ```
///
/// Here `project` is an application-defined `Resource<Vec<String>, String>`.
/// The slot signatures are `Fn() -> Page` for `loading`, `Fn(&T) -> bool` for
/// `empty_if`, `Fn(&T) -> Page` for `empty` and `success`, and `Fn(&E) -> Page`
/// for `error`. The empty predicate runs before the success slot for a
/// successful value. An unset slot renders an empty page.
///
/// Use [`Self::from_latest`] with `Resource::latest_after` when a successful
/// action result should take precedence over the resource value while the
/// resource remains the fallback source.
pub struct ResourcePanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	source: ResourcePanelSource<T, E>,
	slots: ResourceSlots<T, E>,
}

impl<T, E> ResourcePanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	/// Creates a panel backed by `resource`.
	///
	/// The panel reads the resource reactively and initially selects the loading
	/// slot when the resource is loading.
	pub fn new(resource: Resource<T, E>) -> Self {
		Self {
			source: ResourcePanelSource::Resource(resource),
			slots: ResourceSlots {
				loading: None,
				empty_if: None,
				empty: None,
				success: None,
				error: None,
			},
		}
	}

	/// Creates a panel backed by a composed latest resource value.
	///
	/// A [`LatestResourceValue`] can expose a successful action result before
	/// falling back to the original resource. Build one with
	/// [`Resource::latest_after`](crate::reactive::Resource::latest_after).
	pub fn from_latest(latest: LatestResourceValue<T, E>) -> Self {
		Self {
			source: ResourcePanelSource::Latest(latest),
			slots: ResourceSlots {
				loading: None,
				empty_if: None,
				empty: None,
				success: None,
				error: None,
			},
		}
	}

	/// Sets the view rendered while the resource is loading.
	///
	/// The slot has the signature `Fn() -> Page`.
	pub fn loading<F>(mut self, render: F) -> Self
	where
		F: Fn() -> Page + 'static,
	{
		self.slots.loading = Some(Rc::new(render));
		self
	}

	/// Sets the predicate that classifies a successful value as empty.
	///
	/// The predicate has the signature `Fn(&T) -> bool` and is evaluated before
	/// the success slot.
	pub fn empty_if<F>(mut self, predicate: F) -> Self
	where
		F: Fn(&T) -> bool + 'static,
	{
		self.slots.empty_if = Some(Rc::new(predicate));
		self
	}

	/// Sets the view rendered for a successful value classified as empty.
	///
	/// The slot has the signature `Fn(&T) -> Page`.
	pub fn empty<F>(mut self, render: F) -> Self
	where
		F: Fn(&T) -> Page + 'static,
	{
		self.slots.empty = Some(Rc::new(render));
		self
	}

	/// Sets the view rendered for a successful, non-empty value.
	///
	/// The slot has the signature `Fn(&T) -> Page`.
	pub fn success<F>(mut self, render: F) -> Self
	where
		F: Fn(&T) -> Page + 'static,
	{
		self.slots.success = Some(Rc::new(render));
		self
	}

	/// Sets the view rendered when the resource fails.
	///
	/// The slot has the signature `Fn(&E) -> Page`. Error display and redaction
	/// remain application-owned.
	pub fn error<F>(mut self, render: F) -> Self
	where
		F: Fn(&E) -> Page + 'static,
	{
		self.slots.error = Some(Rc::new(render));
		self
	}

	/// Renders the slot for the current resource state reactively.
	pub fn render(&self) -> Page {
		let source = self.source.clone();
		let slots = self.slots.clone();
		Page::reactive(move || render_resource_state(source.get(), &slots))
	}
}

impl<T, E> Component for ResourcePanel<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	fn render(&self) -> Page {
		Self::render(self)
	}

	fn name() -> &'static str {
		"ResourcePanel"
	}
}

impl<T, E> IntoPage for ResourcePanel<T, E>
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
	use super::ResourcePanel;
	use crate::component::{Component, IntoPage, Page};
	use crate::reactive::{ReactiveScope, ResourceState, use_action, use_resource};

	#[test]
	fn resource_panel_selects_loading_empty_success_and_error() {
		ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<Vec<String>, String>(Vec::new()) },
				crate::deps![],
			);
			let panel = ResourcePanel::new(resource)
				.loading(|| Page::text("loading"))
				.empty_if(Vec::is_empty)
				.empty(|_| Page::text("empty"))
				.success(|items| Page::text(format!("success:{}", items.len())))
				.error(|error| Page::text(format!("error:{error}")));
			let page = panel.render();

			resource.set(ResourceState::Loading);
			assert_eq!(page.render_to_string(), "loading");
			resource.set(ResourceState::Success(Vec::new()));
			assert_eq!(page.render_to_string(), "empty");
			resource.set(ResourceState::Success(vec!["one".to_string()]));
			assert_eq!(page.render_to_string(), "success:1");
			resource.set(ResourceState::Error("failed".to_string()));
			assert_eq!(page.render_to_string(), "error:failed");
		});
	}

	#[test]
	fn resource_panel_from_latest_prefers_action_success_over_resource() {
		ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<Vec<String>, String>(Vec::new()) },
				crate::deps![],
			);
			let action =
				use_action(|_: ()| async { Ok::<Vec<String>, String>(vec!["action".to_string()]) });
			let latest = resource.latest_after(action);
			let panel = ResourcePanel::from_latest(latest)
				.empty_if(Vec::is_empty)
				.empty(|_| Page::text("empty"))
				.success(|items| Page::text(items.join(",")))
				.error(|error| Page::text(format!("error:{error}")));
			let page = panel.render();

			resource.set(ResourceState::Success(vec!["resource".to_string()]));
			assert_eq!(page.render_to_string(), "resource");
			action.force_success_for_test(vec!["action".to_string()]);
			assert_eq!(page.render_to_string(), "action");
		});
	}

	#[test]
	fn resource_panel_missing_slots_render_empty_and_exposes_component_apis() {
		assert_eq!(ResourcePanel::<String, String>::name(), "ResourcePanel");

		ReactiveScope::run(|| {
			let resource = use_resource(
				|| async { Ok::<String, String>(String::new()) },
				crate::deps![],
			);
			let panel = ResourcePanel::new(resource);
			assert_eq!(panel.render().render_to_string(), "");
			assert_eq!(panel.into_page().render_to_string(), "");
		});
	}
}
