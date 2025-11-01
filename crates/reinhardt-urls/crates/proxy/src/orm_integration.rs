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

/// Macro to implement OrmReflectable for a model type
///
/// # Examples
///
/// ```ignore
/// use reinhardt_proxy::impl_orm_reflectable;
///
/// #[derive(Clone)]
/// struct User {
///     id: i64,
///     name: String,
///     posts: Vec<Post>,
/// }
///
/// impl_orm_reflectable!(User {
///     fields: {
///         id => Integer,
///         name => String,
///     },
///     relationships: {
///         posts => Collection(Post),
///     }
/// });
/// ```
#[macro_export]
macro_rules! impl_orm_reflectable {
    (
        $model:ty {
            fields: {
                $($field:ident => $field_type:ident),* $(,)?
            },
            relationships: {
                $($rel:ident => $rel_type:tt),* $(,)?
            }
        }
    ) => {
        impl $crate::orm_integration::OrmReflectable for $model {
            fn clone_relationship(&self, name: &str) -> Option<Box<dyn std::any::Any + 'static>> {
                match name {
                    $(
                        stringify!($rel) => Some(Box::new(self.$rel.clone()) as Box<dyn std::any::Any + 'static>),
                    )*
                    _ => None,
                }
            }

            fn get_relationship_mut_ref(&mut self, name: &str) -> Option<&mut dyn std::any::Any> {
                match name {
                    $(
                        stringify!($rel) => Some(&mut self.$rel as &mut dyn std::any::Any),
                    )*
                    _ => None,
                }
            }

            fn get_field_value(&self, name: &str) -> Option<$crate::ScalarValue> {
                match name {
                    $(
                        stringify!($field) => {
                            $crate::field_to_scalar_value!(self.$field, $field_type)
                        },
                    )*
                    _ => None,
                }
            }

            fn set_field_value(&mut self, name: &str, value: $crate::ScalarValue) -> $crate::ProxyResult<()> {
                match name {
                    $(
                        stringify!($field) => {
                            $crate::scalar_value_to_field!(self.$field, value, $field_type)
                        },
                    )*
                    _ => Err($crate::ProxyError::AttributeNotFound(name.to_string())),
                }
            }
        }
    };
}

/// Helper macro to convert field values to ScalarValue
#[macro_export]
macro_rules! field_to_scalar_value {
	($field:expr, Integer) => {
		Some($crate::ScalarValue::Integer($field as i64))
	};
	($field:expr, String) => {
		Some($crate::ScalarValue::String($field.clone()))
	};
	($field:expr, Float) => {
		Some($crate::ScalarValue::Float($field as f64))
	};
	($field:expr, Boolean) => {
		Some($crate::ScalarValue::Boolean($field))
	};
}

/// Helper macro to convert ScalarValue to field values
#[macro_export]
macro_rules! scalar_value_to_field {
	($field:expr, $value:expr, Integer) => {{
		$field = $value.as_integer()? as _;
		Ok(())
	}};
	($field:expr, $value:expr, String) => {{
		$field = $value.as_string()?;
		Ok(())
	}};
	($field:expr, $value:expr, Float) => {{
		$field = $value.as_float()? as _;
		Ok(())
	}};
	($field:expr, $value:expr, Boolean) => {{
		$field = $value.as_boolean()?;
		Ok(())
	}};
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reflection::Reflectable;

	#[derive(Clone)]
	struct TestPost {
		id: i64,
		title: String,
	}

	impl_orm_reflectable!(TestPost {
		fields: {
			id => Integer,
			title => String,
		},
		relationships: {}
	});

	#[test]
	fn test_orm_reflectable_get_attribute() {
		let post = TestPost {
			id: 1,
			title: "Test Post".to_string(),
		};

		let id_value = post.get_attribute("id");
		assert!(id_value.is_some());
		assert_eq!(id_value.unwrap().as_integer().unwrap(), 1);

		let title_value = post.get_attribute("title");
		assert!(title_value.is_some());
		assert_eq!(title_value.unwrap().as_string().unwrap(), "Test Post");
	}

	#[test]
	fn test_orm_reflectable_set_attribute() {
		let mut post = TestPost {
			id: 1,
			title: "Old Title".to_string(),
		};

		post.set_attribute("title", ScalarValue::String("New Title".to_string()))
			.unwrap();
		assert_eq!(post.title, "New Title");

		post.set_attribute("id", ScalarValue::Integer(42)).unwrap();
		assert_eq!(post.id, 42);
	}
}
