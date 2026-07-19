//! Development-only metadata carried by [`Page`](super::Page) values.

use std::any::Any;
use std::sync::Arc;

/// Opaque development metadata attached to a page template.
#[derive(Clone)]
pub struct DevTemplateMetadata(Arc<dyn Any + Send + Sync>);

impl DevTemplateMetadata {
	/// Wraps a concrete metadata value.
	pub fn new<T>(value: T) -> Self
	where
		T: Any + Send + Sync,
	{
		Self(Arc::new(value))
	}

	/// Returns the metadata as `T` when the stored type matches.
	pub fn downcast_ref<T>(&self) -> Option<&T>
	where
		T: Any,
	{
		self.0.downcast_ref()
	}
}

impl std::fmt::Debug for DevTemplateMetadata {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.debug_tuple("DevTemplateMetadata").finish()
	}
}
