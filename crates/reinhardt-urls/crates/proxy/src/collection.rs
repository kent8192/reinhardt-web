//! Collection association proxies for one-to-many and many-to-many relationships

pub mod aggregations;
pub mod operations;
#[cfg(test)]
mod tests;

pub use aggregations::CollectionAggregations;
pub use operations::CollectionOperations;

use crate::proxy::ScalarValue;
use crate::{ProxyError, ProxyResult};

/// Collection proxy for accessing multiple related objects' attributes
///
/// Used for one-to-many and many-to-many relationships where the proxy
/// returns a collection of scalar values.
///
/// ## Example
///
/// ```rust,ignore
// User has many posts, access all post titles directly
/// let titles_proxy = CollectionProxy::new("posts", "title");
/// let titles: Vec<String> = titles_proxy.get_values(&user).await?;
/// ```
#[derive(Clone)]
pub struct CollectionProxy {
	/// Name of the relationship
	pub relationship: String,

	/// Name of the attribute on the related objects
	pub attribute: String,

	/// Whether to remove duplicates
	pub unique: bool,

	/// Factory for creating new instances from scalar values
	pub factory: Option<std::sync::Arc<dyn crate::reflection::ReflectableFactory>>,
}

impl CollectionProxy {
	/// Create a new collection proxy
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title");
	/// assert_eq!(proxy.relationship, "posts");
	/// assert_eq!(proxy.attribute, "title");
	/// assert!(!proxy.unique);
	/// ```
	pub fn new(relationship: &str, attribute: &str) -> Self {
		Self {
			relationship: relationship.to_string(),
			attribute: attribute.to_string(),
			unique: false,
			factory: None,
		}
	}
	/// Create a collection proxy that removes duplicates
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::unique("tags", "name");
	/// assert_eq!(proxy.relationship, "tags");
	/// assert_eq!(proxy.attribute, "name");
	/// assert!(proxy.unique);
	/// ```
	pub fn unique(relationship: &str, attribute: &str) -> Self {
		Self {
			relationship: relationship.to_string(),
			attribute: attribute.to_string(),
			unique: true,
			factory: None,
		}
	}

	/// Create a collection proxy with a factory for creating instances
	pub fn with_factory(
		relationship: &str,
		attribute: &str,
		factory: std::sync::Arc<dyn crate::reflection::ReflectableFactory>,
	) -> Self {
		Self {
			relationship: relationship.to_string(),
			attribute: attribute.to_string(),
			unique: false,
			factory: Some(factory),
		}
	}

	/// Set the factory for this proxy
	pub fn set_factory(
		&mut self,
		factory: std::sync::Arc<dyn crate::reflection::ReflectableFactory>,
	) {
		self.factory = Some(factory);
	}
	/// Get collection of values from related objects
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("posts", "title");
	// Assuming \`user\` implements Reflectable
	// let titles = proxy.get_values(&user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_values<T>(&self, source: &T) -> ProxyResult<Vec<ScalarValue>>
	where
		T: crate::reflection::Reflectable,
	{
		// 1. Access the relationship on source
		let relationship = source
			.get_relationship(&self.relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

		// 2. Downcast to Vec<Box<dyn Reflectable>>
		let collection = crate::reflection::downcast_relationship::<
			Vec<Box<dyn crate::reflection::Reflectable>>,
		>(relationship)?;

		// 3. Extract the attribute from each item
		let mut values = Vec::new();
		for item in collection.iter() {
			let value = item
				.get_attribute(&self.attribute)
				.ok_or_else(|| ProxyError::AttributeNotFound(self.attribute.clone()))?;
			values.push(value);
		}

		// 4. Optionally remove duplicates
		if self.unique {
			values.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
			values.dedup_by(|a, b| format!("{:?}", a) == format!("{:?}", b));
		}

		Ok(values)
	}
	/// Set collection of values by creating/updating related objects
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("tags", "name");
	/// let values = vec![ScalarValue::String("rust".to_string())];
	// let mut user = ...;
	// proxy.set_values(&mut user, values).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn set_values<T>(&self, source: &mut T, values: Vec<ScalarValue>) -> ProxyResult<()>
	where
		T: crate::reflection::Reflectable,
	{
		// 1. Check if factory is configured
		let factory = self
			.factory
			.as_ref()
			.ok_or(ProxyError::FactoryNotConfigured)?;

		// 2. Access the relationship
		let relationship = source
			.get_relationship_mut(&self.relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

		// 3. Downcast to Vec<Box<dyn Reflectable>>
		let collection = relationship
			.downcast_mut::<Vec<Box<dyn crate::reflection::Reflectable>>>()
			.ok_or_else(|| ProxyError::TypeMismatch {
				expected: "Vec<Box<dyn Reflectable>>".to_string(),
				actual: "unknown".to_string(),
			})?;

		// 4. Clear existing collection
		collection.clear();

		// 5. Create new objects from scalar values and add to collection
		for value in values {
			let new_object = factory.create_from_scalar(&self.attribute, value)?;
			collection.push(new_object);
		}

		Ok(())
	}
	/// Append a value to the collection
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("tags", "name");
	// let mut user = ...;
	// proxy.append(&mut user, ScalarValue::String("new_tag".to_string())).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn append<T>(&self, source: &mut T, value: ScalarValue) -> ProxyResult<()>
	where
		T: crate::reflection::Reflectable,
	{
		// 1. Check if factory is configured
		let factory = self
			.factory
			.as_ref()
			.ok_or(ProxyError::FactoryNotConfigured)?;

		// 2. Access the relationship
		let relationship = source
			.get_relationship_mut(&self.relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

		// 3. Downcast to Vec<Box<dyn Reflectable>>
		let collection = relationship
			.downcast_mut::<Vec<Box<dyn crate::reflection::Reflectable>>>()
			.ok_or_else(|| ProxyError::TypeMismatch {
				expected: "Vec<Box<dyn Reflectable>>".to_string(),
				actual: "unknown".to_string(),
			})?;

		// 4. Create new object from scalar value
		let new_object = factory.create_from_scalar(&self.attribute, value)?;

		// 5. Append to collection
		collection.push(new_object);

		Ok(())
	}
	/// Remove a value from the collection
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("tags", "name");
	// let mut user = ...;
	// proxy.remove(&mut user, ScalarValue::String("old_tag".to_string())).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove<T>(&self, source: &mut T, value: ScalarValue) -> ProxyResult<()>
	where
		T: crate::reflection::Reflectable,
	{
		// 1. Access the relationship
		let relationship = source
			.get_relationship_mut(&self.relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

		// 2. Downcast to Vec<Box<dyn Reflectable>>
		let collection = relationship
			.downcast_mut::<Vec<Box<dyn crate::reflection::Reflectable>>>()
			.ok_or_else(|| ProxyError::TypeMismatch {
				expected: "Vec<Box<dyn Reflectable>>".to_string(),
				actual: "unknown".to_string(),
			})?;

		// 3. Find and remove items with matching attribute value
		collection.retain(|item| {
			item.get_attribute(&self.attribute)
				.map(|v| v != value)
				.unwrap_or(true)
		});

		Ok(())
	}
	/// Check if the collection contains a value
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("tags", "name");
	// let user = ...;
	// let has_tag = proxy.contains(&user, ScalarValue::String("rust".to_string())).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn contains<T>(&self, source: &T, value: ScalarValue) -> ProxyResult<bool>
	where
		T: crate::reflection::Reflectable,
	{
		let values = self.get_values(source).await?;
		Ok(values.contains(&value))
	}
	/// Get the count of items in the collection
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("posts", "title");
	// let user = ...;
	// let count = proxy.count(&user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn count<T>(&self, source: &T) -> ProxyResult<usize>
	where
		T: crate::reflection::Reflectable,
	{
		// 1. Access the relationship
		let relationship = source
			.get_relationship(&self.relationship)
			.ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

		// 2. Downcast to Vec<Box<dyn Reflectable>>
		let collection = crate::reflection::downcast_relationship::<
			Vec<Box<dyn crate::reflection::Reflectable>>,
		>(relationship)?;

		// 3. Return the count
		Ok(collection.len())
	}
	/// Filter collection by a condition on the proxy attribute
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, query::{FilterCondition, FilterOp}};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("posts", "status");
	/// let condition = FilterCondition::new("status", FilterOp::eq("published"));
	// let user = ...;
	// let filtered = proxy.filter(&user, condition).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn filter<T>(
		&self,
		source: &T,
		condition: crate::query::FilterCondition,
	) -> ProxyResult<Vec<ScalarValue>>
	where
		T: crate::reflection::Reflectable,
	{
		// Get all values first
		let values = self.get_values(source).await?;

		// Filter using the condition
		let filtered: Vec<ScalarValue> = values
			.into_iter()
			.filter(|v| condition.matches(v))
			.collect();

		Ok(filtered)
	}
	/// Filter collection using a custom predicate
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::{CollectionProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = CollectionProxy::new("posts", "views");
	// let user = ...;
	// let popular = proxy.filter_by(&user, |v| {
	//     matches!(v, ScalarValue::Integer(n) if *n > 1000)
	// }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn filter_by<T, F>(&self, source: &T, predicate: F) -> ProxyResult<Vec<ScalarValue>>
	where
		T: crate::reflection::Reflectable,
		F: Fn(&ScalarValue) -> bool,
	{
		// Get all values first
		let values = self.get_values(source).await?;

		// Filter using the predicate
		let filtered: Vec<ScalarValue> = values.into_iter().filter(|v| predicate(v)).collect();

		Ok(filtered)
	}
}

// Manual Debug implementation for CollectionProxy
impl std::fmt::Debug for CollectionProxy {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CollectionProxy")
			.field("relationship", &self.relationship)
			.field("attribute", &self.attribute)
			.field("unique", &self.unique)
			.field("factory", &self.factory.as_ref().map(|_| "Some(<factory>)"))
			.finish()
	}
}
