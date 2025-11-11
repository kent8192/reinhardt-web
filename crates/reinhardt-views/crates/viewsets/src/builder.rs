/// ViewSetBuilder - converts ViewSet to Handler with action mapping
use crate::ViewSet;
use hyper::Method;
use reinhardt_core::{Handler, http::Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Builder for creating a Handler from a ViewSet
pub struct ViewSetBuilder<V: ViewSet> {
	viewset: Arc<V>,
	actions: HashMap<Method, String>,
	name: Option<String>,
	suffix: Option<String>,
}

impl<V: ViewSet + 'static> ViewSetBuilder<V> {
	/// Create a new builder with a ViewSet
	pub fn new(viewset: V) -> Self {
		Self {
			viewset: Arc::new(viewset),
			actions: HashMap::new(),
			name: None,
			suffix: None,
		}
	}

	/// Set action mappings (HTTP method -> action name)
	pub fn with_actions(mut self, actions: HashMap<Method, String>) -> Self {
		self.actions = actions;
		self
	}

	/// Add a single action mapping
	pub fn action(mut self, method: Method, action_name: impl Into<String>) -> Self {
		self.actions.insert(method, action_name.into());
		self
	}

	/// Set a custom name (mutually exclusive with suffix)
	pub fn with_name(mut self, name: impl Into<String>) -> Result<Self> {
		if self.suffix.is_some() {
			return Err(reinhardt_core::exception::Error::Http(format!(
				"{}() received both `name` and `suffix`, which are mutually exclusive arguments.",
				std::any::type_name::<V>()
			)));
		}
		self.name = Some(name.into());
		Ok(self)
	}

	/// Set a custom suffix (mutually exclusive with name)
	pub fn with_suffix(mut self, suffix: impl Into<String>) -> Result<Self> {
		if self.name.is_some() {
			return Err(reinhardt_core::exception::Error::Http(format!(
				"{}() received both `name` and `suffix`, which are mutually exclusive arguments.",
				std::any::type_name::<V>()
			)));
		}
		self.suffix = Some(suffix.into());
		Ok(self)
	}

	/// Build the Handler
	pub fn build(self) -> Result<Arc<dyn Handler>> {
		// Validate that actions are not empty
		if self.actions.is_empty() {
			return Err(reinhardt_core::exception::Error::Http(
				"The `actions` argument must be provided when calling `.as_view()` on a ViewSet. \
                 For example `.as_view({'get': 'list'})`"
					.to_string(),
			));
		}

		Ok(Arc::new(crate::handler::ViewSetHandler::new(
			self.viewset,
			self.actions,
			self.name,
			self.suffix,
		)))
	}
}

/// Helper macro to create action mappings
#[macro_export]
macro_rules! viewset_actions {
    ($($method:ident => $action:expr),* $(,)?) => {{
        let mut actions = std::collections::HashMap::new();
        $(
            actions.insert(hyper::Method::$method, $action.to_string());
        )*
        actions
    }};
}
