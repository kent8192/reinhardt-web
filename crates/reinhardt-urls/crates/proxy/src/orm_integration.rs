//! ORM integration for association proxies
//!
//! Provides macros and utilities to integrate association proxies with reinhardt-orm models.

use crate::{ProxyError, ProxyResult, ScalarValue};
use std::any::Any;

/// Helper trait for ORM models to provide reflection capabilities
///
/// This trait should be implemented by ORM models to enable proxy access.
/// Use the `impl_orm_reflectable!` macro for automatic implementation.
pub trait OrmReflectable: 'static {
	/// Get a cloned relationship by name
	///
	/// Returns a cloned copy of the relationship data as a boxed Any.
	fn clone_relationship(&self, name: &str) -> Option<Box<dyn Any + 'static>>;

	/// Get a mutable relationship by name
	fn get_relationship_mut_ref(&mut self, name: &str) -> Option<&mut dyn Any>;

	/// Get scalar attribute value by field name
	fn get_field_value(&self, name: &str) -> Option<ScalarValue>;

	/// Set scalar attribute value by field name
	fn set_field_value(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()>;
}

/// Automatically implement Reflectable for types that implement OrmReflectable
impl<T: OrmReflectable> crate::reflection::Reflectable for T {
	fn get_relationship(&self, name: &str) -> Option<Box<dyn Any + 'static>> {
		self.clone_relationship(name)
	}

	fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any> {
		self.get_relationship_mut_ref(name)
	}

	fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
		self.get_field_value(name)
	}

	fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
		self.set_field_value(name, value)
	}

	fn set_relationship_attribute(
		&mut self,
		relationship: &str,
		attribute: &str,
		value: ScalarValue,
	) -> ProxyResult<()> {
		// Get mutable reference to the relationship
		let rel = self
			.get_relationship_mut(relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(relationship.to_string()))?;

		// Downcast to Box<dyn Reflectable> for scalar relationships
		if let Some(related) = rel.downcast_mut::<Box<dyn crate::reflection::Reflectable>>() {
			related.set_attribute(attribute, value)?;
			Ok(())
		} else {
			Err(ProxyError::TypeMismatch {
				expected: "Box<dyn Reflectable>".to_string(),
				actual: "unknown".to_string(),
			})
		}
	}
}
