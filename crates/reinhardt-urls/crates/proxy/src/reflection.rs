//! Reflection utilities for association proxies
//!
//! Provides runtime introspection capabilities for accessing relationships
//! and attributes on model objects.

use crate::{ProxyError, ProxyResult, ScalarValue};
use std::any::Any;

/// Trait for objects that support reflection-based attribute access
///
/// This trait enables association proxies to access relationships and attributes
/// at runtime without compile-time type information.
///
/// ## Example
///
/// ```rust,ignore
/// struct User {
///     id: i64,
///     name: String,
///     posts: Vec<Post>,
/// }
///
/// impl Reflectable for User {
///     fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
///         match name {
///             "posts" => Some(Box::new(self.posts.clone())),
///             _ => None,
///         }
///     }
///
///     fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
///         match name {
///             "name" => Some(ScalarValue::String(self.name.clone())),
///             "id" => Some(ScalarValue::Integer(self.id)),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait Reflectable {
	/// Get a relationship by name
	///
	/// Returns the relationship collection/object as a boxed Any trait object.
	/// The caller is responsible for downcasting to the appropriate type.
	fn get_relationship(&self, name: &str) -> Option<Box<dyn Any + 'static>>;

	/// Get a mutable reference to a relationship by name
	fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any>;

	/// Get an attribute value by name
	///
	/// Returns the attribute as a ScalarValue, or None if not found.
	fn get_attribute(&self, name: &str) -> Option<ScalarValue>;

	/// Set an attribute value by name
	///
	/// Returns an error if the attribute doesn't exist or the type is incompatible.
	fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()>;

	/// Get an attribute from a related object
	///
	/// This is a convenience method for scalar proxies to access nested attributes.
	fn get_relationship_attribute(
		&self,
		relationship: &str,
		attribute: &str,
	) -> ProxyResult<Option<ScalarValue>> {
		let rel = self
			.get_relationship(relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(relationship.to_string()))?;

		let related = crate::reflection::downcast_relationship::<Box<dyn Reflectable>>(rel)?;
		Ok(related.get_attribute(attribute))
	}

	/// Set an attribute on a related object
	///
	/// This is a convenience method for scalar proxies to modify nested attributes.
	fn set_relationship_attribute(
		&mut self,
		relationship: &str,
		attribute: &str,
		value: ScalarValue,
	) -> ProxyResult<()>;

	/// Check if a relationship exists
	fn has_relationship(&self, name: &str) -> bool {
		self.get_relationship(name).is_some()
	}

	/// Check if an attribute exists
	fn has_attribute(&self, name: &str) -> bool {
		self.get_attribute(name).is_some()
	}

	/// Get a reference to self as Any for downcasting
	fn as_any(&self) -> &dyn Any {
		panic!("as_any() not implemented for this type");
	}
}

/// Factory trait for creating Reflectable instances from ScalarValue
///
/// This trait enables CollectionProxy to create new instances of related
/// objects from scalar values (strings, integers, etc.).
///
/// # Examples
///
/// ```ignore
/// use reinhardt_proxy::{ReflectableFactory, Reflectable, ScalarValue, ProxyResult};
///
/// struct TagFactory;
///
/// impl ReflectableFactory for TagFactory {
///     fn create_from_scalar(
///         &self,
///         attribute_name: &str,
///         value: ScalarValue,
///     ) -> ProxyResult<Box<dyn Reflectable>> {
///         match attribute_name {
///             "name" => {
///                 if let ScalarValue::String(name) = value {
///                     Ok(Box::new(Tag { id: None, name }))
///                 } else {
///                     Err(ProxyError::TypeMismatch {
///                         expected: "String".to_string(),
///                         actual: format!("{:?}", value),
///                     })
///                 }
///             }
///             _ => Err(ProxyError::AttributeNotFound(attribute_name.to_string())),
///         }
///     }
/// }
/// ```
pub trait ReflectableFactory: Send + Sync {
	/// Create a new Reflectable instance with the given attribute value
	///
	/// # Arguments
	///
	/// * `attribute_name` - The name of the attribute to set
	/// * `value` - The scalar value to set for the attribute
	///
	/// # Returns
	///
	/// A boxed Reflectable instance with the attribute set, or an error
	/// if the attribute doesn't exist or the value type is incompatible.
	fn create_from_scalar(
		&self,
		attribute_name: &str,
		value: ScalarValue,
	) -> ProxyResult<Box<dyn Reflectable>>;
}

/// Trait for collections that can be accessed through association proxies
///
/// This trait provides a unified interface for different collection types
/// (Vec, HashSet, HashMap, etc.) to be used with association proxies.
pub trait ProxyCollection {
	/// The type of items in the collection
	type Item;

	/// Get all items in the collection
	fn items(&self) -> Vec<&Self::Item>;

	/// Add an item to the collection
	fn add(&mut self, item: Self::Item);

	/// Remove an item from the collection
	fn remove(&mut self, item: &Self::Item) -> bool;

	/// Check if the collection contains an item
	fn contains(&self, item: &Self::Item) -> bool;

	/// Get the number of items in the collection
	fn len(&self) -> usize;

	/// Check if the collection is empty
	fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Clear all items from the collection
	fn clear(&mut self);
}

/// Implementation of ProxyCollection for Vec
impl<T> ProxyCollection for Vec<T>
where
	T: PartialEq,
{
	type Item = T;

	fn items(&self) -> Vec<&Self::Item> {
		self.iter().collect()
	}

	fn add(&mut self, item: Self::Item) {
		self.push(item);
	}

	fn remove(&mut self, item: &Self::Item) -> bool {
		if let Some(pos) = self.iter().position(|x| x == item) {
			self.remove(pos);
			true
		} else {
			false
		}
	}

	fn contains(&self, item: &Self::Item) -> bool {
		self.iter().any(|x| x == item)
	}

	fn len(&self) -> usize {
		Vec::len(self)
	}

	fn clear(&mut self) {
		Vec::clear(self);
	}
}

/// Helper trait to extract scalar values from objects
///
/// This trait should be implemented by model types to enable
/// attribute extraction through association proxies.
pub trait AttributeExtractor {
	/// Extract a scalar value for the given attribute name
	fn extract_attribute(&self, name: &str) -> ProxyResult<ScalarValue>;
}
/// Helper function to downcast a relationship to a specific collection type
///
pub fn downcast_relationship<T: 'static>(relationship: Box<dyn Any>) -> ProxyResult<Box<T>> {
	relationship
		.downcast::<T>()
		.map_err(|_| ProxyError::TypeMismatch {
			expected: std::any::type_name::<T>().to_string(),
			actual: "unknown".to_string(),
		})
}
/// Helper function to extract values from a collection through an attribute
///
pub fn extract_collection_values<T>(
	collection: &[T],
	attribute_extractor: impl Fn(&T) -> ProxyResult<ScalarValue>,
) -> ProxyResult<Vec<ScalarValue>> {
	collection
		.iter()
		.map(attribute_extractor)
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, PartialEq)]
	struct TestModel {
		id: i64,
		name: String,
	}

	impl Reflectable for TestModel {
		fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any + 'static>> {
			None
		}

		fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
			None
		}

		fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
			match name {
				"id" => Some(ScalarValue::Integer(self.id)),
				"name" => Some(ScalarValue::String(self.name.clone())),
				_ => None,
			}
		}

		fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
			match name {
				"id" => {
					self.id = value.as_integer()?;
					Ok(())
				}
				"name" => {
					self.name = value.as_string()?;
					Ok(())
				}
				_ => Err(ProxyError::AttributeNotFound(name.to_string())),
			}
		}

		fn set_relationship_attribute(
			&mut self,
			relationship: &str,
			_attribute: &str,
			_value: ScalarValue,
		) -> ProxyResult<()> {
			Err(ProxyError::RelationshipNotFound(relationship.to_string()))
		}
	}

	#[test]
	fn test_reflectable_get_attribute() {
		let model = TestModel {
			id: 42,
			name: "test".to_string(),
		};

		let id = model.get_attribute("id");
		assert!(id.is_some());
		assert_eq!(id.unwrap().as_integer().unwrap(), 42);

		let name = model.get_attribute("name");
		assert!(name.is_some());
		assert_eq!(name.unwrap().as_string().unwrap(), "test");
	}

	#[test]
	fn test_reflectable_set_attribute() {
		let mut model = TestModel {
			id: 42,
			name: "test".to_string(),
		};

		model
			.set_attribute("name", ScalarValue::String("new_name".to_string()))
			.unwrap();
		assert_eq!(model.name, "new_name");

		model
			.set_attribute("id", ScalarValue::Integer(100))
			.unwrap();
		assert_eq!(model.id, 100);
	}

	#[test]
	fn test_reflectable_has_attribute() {
		let model = TestModel {
			id: 42,
			name: "test".to_string(),
		};

		assert!(model.has_attribute("id"));
		assert!(model.has_attribute("name"));
		assert!(!model.has_attribute("nonexistent"));
	}

	#[test]
	fn test_proxy_collection_vec() {
		use super::ProxyCollection;

		let mut vec: Vec<i32> = vec![1, 2, 3];

		assert_eq!(ProxyCollection::len(&vec), 3);
		assert!(ProxyCollection::contains(&vec, &2));

		ProxyCollection::add(&mut vec, 4);
		assert_eq!(ProxyCollection::len(&vec), 4);

		assert!(ProxyCollection::remove(&mut vec, &2));
		assert_eq!(ProxyCollection::len(&vec), 3);
		assert!(!ProxyCollection::contains(&vec, &2));

		ProxyCollection::clear(&mut vec);
		assert!(ProxyCollection::is_empty(&vec));
	}

	#[test]
	fn test_extract_collection_values() {
		let models = vec![
			TestModel {
				id: 1,
				name: "first".to_string(),
			},
			TestModel {
				id: 2,
				name: "second".to_string(),
			},
		];

		let names = extract_collection_values(&models, |m| Ok(ScalarValue::String(m.name.clone())))
			.unwrap();

		assert_eq!(names.len(), 2);
		assert_eq!(names[0].as_string().unwrap(), "first");
		assert_eq!(names[1].as_string().unwrap(), "second");
	}
}
