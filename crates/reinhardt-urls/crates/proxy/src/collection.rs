//! Collection association proxies for one-to-many and many-to-many relationships

pub mod aggregations;
pub mod operations;

pub use aggregations::CollectionAggregations;
pub use operations::CollectionOperations;

use crate::proxy::ScalarValue;
use crate::{ProxyError, ProxyResult};
use serde::{Deserialize, Serialize};

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

	/// Whether to deduplicate values
	pub deduplicate: bool,

	/// Loading strategy for the relationship
	pub loading_strategy: Option<crate::LoadingStrategy>,

	/// Factory for creating new instances from scalar values
	pub factory: Option<std::sync::Arc<dyn crate::reflection::ReflectableFactory>>,

	/// Whether caching is enabled
	caching: bool,

	/// Cache time-to-live in seconds
	cache_ttl: Option<u64>,

	/// Whether streaming is enabled
	streaming: bool,

	/// Whether triggers are enabled
	triggers: bool,

	/// Trigger events to monitor
	trigger_events: Vec<String>,

	/// Stored procedure name
	stored_procedure: Option<String>,

	/// Stored procedure parameters
	procedure_params: Vec<(String, String)>,

	/// Target database name
	database: Option<String>,

	/// Fallback database name
	fallback_database: Option<String>,

	/// Whether async loading is enabled
	async_loading: bool,

	/// Whether concurrent access is supported
	concurrent_access: bool,
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
			deduplicate: false,
			loading_strategy: None,
			factory: None,
			caching: false,
			cache_ttl: None,
			streaming: false,
			triggers: false,
			trigger_events: Vec::new(),
			stored_procedure: None,
			procedure_params: Vec::new(),
			database: None,
			fallback_database: None,
			async_loading: false,
			concurrent_access: false,
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
			deduplicate: false,
			loading_strategy: None,
			factory: None,
			caching: false,
			cache_ttl: None,
			streaming: false,
			triggers: false,
			trigger_events: Vec::new(),
			stored_procedure: None,
			procedure_params: Vec::new(),
			database: None,
			fallback_database: None,
			async_loading: false,
			concurrent_access: false,
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
			deduplicate: false,
			loading_strategy: None,
			factory: Some(factory),
			caching: false,
			cache_ttl: None,
			streaming: false,
			triggers: false,
			trigger_events: Vec::new(),
			stored_procedure: None,
			procedure_params: Vec::new(),
			database: None,
			fallback_database: None,
			async_loading: false,
			concurrent_access: false,
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

	// ========== Accessor methods ==========

	/// Get the relationship name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title");
	/// assert_eq!(proxy.relationship(), "posts");
	/// ```
	pub fn relationship(&self) -> &str {
		&self.relationship
	}

	/// Get the attribute name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title");
	/// assert_eq!(proxy.attribute(), "title");
	/// ```
	pub fn attribute(&self) -> &str {
		&self.attribute
	}

	/// Check if this proxy removes duplicates
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::unique("tags", "name");
	/// assert!(proxy.is_unique());
	///
	/// let proxy2 = CollectionProxy::new("posts", "title");
	/// assert!(!proxy2.is_unique());
	/// ```
	pub fn is_unique(&self) -> bool {
		self.unique
	}

	/// Check if this proxy deduplicates values
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("tags", "name").with_deduplication(true);
	/// assert!(proxy.deduplicates());
	/// ```
	pub fn deduplicates(&self) -> bool {
		self.deduplicate
	}

	// ========== Builder pattern methods ==========

	/// Set whether to remove duplicates (builder pattern)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("tags", "name").with_unique(true);
	/// assert!(proxy.is_unique());
	/// ```
	pub fn with_unique(mut self, unique: bool) -> Self {
		self.unique = unique;
		self
	}

	/// Set whether to deduplicate values (builder pattern)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("tags", "name").with_deduplication(true);
	/// assert!(proxy.deduplicates());
	/// ```
	pub fn with_deduplication(mut self, deduplicate: bool) -> Self {
		self.deduplicate = deduplicate;
		self
	}

	/// Set the loading strategy (builder pattern)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{CollectionProxy, LoadingStrategy};
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_loading_strategy(LoadingStrategy::Joined);
	/// ```
	pub fn with_loading_strategy(mut self, strategy: crate::LoadingStrategy) -> Self {
		self.loading_strategy = Some(strategy);
		self
	}

	/// Filter collection where any item matches the condition
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("tags", "name");
	/// let result = proxy.filter_with_any("name", "rust").await;
	/// ```
	pub async fn filter_with_any(&self, _field: &str, _value: &str) -> ProxyResult<Self> {
		// TODO: Implement filter_with_any logic
		Ok(self.clone())
	}

	/// Filter collection where item has specific relationship
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title");
	/// let result = proxy.filter_with_has("comments", "Alice").await;
	/// ```
	pub async fn filter_with_has(&self, _field: &str, _value: &str) -> ProxyResult<Self> {
		// TODO: Implement filter_with_has logic
		Ok(self.clone())
	}

	/// Configure cascade behavior for related operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_cascade(true);
	/// ```
	pub fn with_cascade(self, _cascade: bool) -> Self {
		// TODO: Implement cascade configuration
		self
	}

	/// Merge with another collection proxy
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy1 = CollectionProxy::new("posts", "title");
	/// let proxy2 = CollectionProxy::new("drafts", "title");
	/// let merged = proxy1.merge(proxy2);
	/// ```
	pub fn merge(self, _other: Self) -> Self {
		// TODO: Implement merge logic
		self
	}

	/// Set batch size for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_batch_size(100);
	/// ```
	pub fn with_batch_size(self, _batch_size: usize) -> Self {
		// TODO: Implement batch size configuration
		self
	}

	/// Enable async loading for collection
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_async_loading(true);
	/// ```
	pub fn with_async_loading(mut self, async_loading: bool) -> Self {
		self.async_loading = async_loading;
		self
	}

	/// Enable streaming for large collections
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("logs", "message")
	///     .with_streaming(true);
	/// ```
	pub fn with_streaming(mut self, streaming: bool) -> Self {
		self.streaming = streaming;
		self
	}

	/// Enable caching for collection queries
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_caching(true);
	/// ```
	pub fn with_caching(mut self, caching: bool) -> Self {
		self.caching = caching;
		self
	}

	/// Configure database for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_database("analytics");
	/// ```
	pub fn with_database(mut self, database: &str) -> Self {
		self.database = Some(database.to_string());
		self
	}

	/// Set memory limit for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_memory_limit(1024 * 1024);
	/// ```
	pub fn with_memory_limit(self, _memory_limit: usize) -> Self {
		// TODO: Implement memory limit configuration
		self
	}

	/// Configure stored procedure for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_stored_procedure("get_posts");
	/// ```
	pub fn with_stored_procedure(mut self, procedure: &str) -> Self {
		self.stored_procedure = Some(procedure.to_string());
		self
	}

	/// Configure triggers for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_triggers(true);
	/// ```
	pub fn with_triggers(mut self, triggers: bool) -> Self {
		self.triggers = triggers;
		self
	}

	/// Enable version tracking for collection items
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_version_tracking(true);
	/// ```
	pub fn with_version_tracking(self, _version_tracking: bool) -> Self {
		// TODO: Implement version tracking configuration
		self
	}

	/// Configure view for collection operations
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_view("published_posts");
	/// ```
	pub fn with_view(self, _view: &str) -> Self {
		// TODO: Implement view configuration
		self
	}

	/// Bulk insert multiple items into the collection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("users", "email");
	/// let items = vec!["alice@example.com", "bob@example.com"];
	/// let _ = proxy.bulk_insert(&items);
	/// ```
	pub fn bulk_insert<I>(&self, _items: &[I]) -> ProxyResult<()> {
		// TODO: Implement bulk insert logic
		Ok(())
	}

	/// Clear all items from the collection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("tags", "name");
	/// let _ = proxy.clear();
	/// ```
	pub fn clear(&self) -> ProxyResult<()> {
		// TODO: Implement clear logic
		Ok(())
	}

	/// Check if this proxy targets a database view
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("user_view", "name")
	///     .with_view("user_summary");
	/// assert!(!proxy.is_view()); // Returns false until view tracking is implemented
	/// ```
	pub fn is_view(&self) -> bool {
		// TODO: Implement view detection logic
		false
	}

	/// Update items with version tracking
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("documents", "content")
	///     .with_version_tracking(true);
	/// let _ = proxy.update_with_version("new_content", 1);
	/// ```
	pub fn update_with_version<V>(&self, _value: V, _version: i64) -> ProxyResult<()> {
		// TODO: Implement versioned update logic
		Ok(())
	}

	/// Configure cache time-to-live in seconds
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("products", "price")
	///     .with_caching(true)
	///     .with_cache_ttl(3600); // 1 hour
	/// ```
	pub fn with_cache_ttl(mut self, ttl: u64) -> Self {
		self.cache_ttl = Some(ttl);
		self
	}

	/// Configure chunk size for batch operations
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("orders", "total")
	///     .with_batch_size(100)
	///     .with_chunk_size(10); // Process 10 items at a time
	/// ```
	pub fn with_chunk_size(self, _chunk_size: usize) -> Self {
		// TODO: Implement chunk size configuration
		self
	}

	/// Configure concurrent access settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("sessions", "data")
	///     .with_concurrent_access(true);
	/// ```
	pub fn with_concurrent_access(mut self, concurrent: bool) -> Self {
		self.concurrent_access = concurrent;
		self
	}

	/// Configure fallback database for read operations
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("users", "name")
	///     .with_database("primary")
	///     .with_fallback_database("replica");
	/// ```
	pub fn with_fallback_database(mut self, database: &str) -> Self {
		self.fallback_database = Some(database.to_string());
		self
	}

	/// Configure parameters for stored procedures
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("reports", "data")
	///     .with_stored_procedure("generate_report")
	///     .with_procedure_params(&[("start_date", "2024-01-01"), ("end_date", "2024-12-31")]);
	/// ```
	pub fn with_procedure_params(mut self, params: &[(&str, &str)]) -> Self {
		self.procedure_params = params
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect();
		self
	}

	/// Configure trigger events to monitor
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("audit_logs", "action")
	///     .with_triggers(true)
	///     .with_trigger_events(&["INSERT", "UPDATE", "DELETE"]);
	/// ```
	pub fn with_trigger_events(mut self, events: &[&str]) -> Self {
		self.trigger_events = events.iter().map(|e| e.to_string()).collect();
		self
	}

	// Getter methods

	/// Check if caching is enabled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_caching(true);
	/// assert!(proxy.is_cached());
	/// ```
	pub fn is_cached(&self) -> bool {
		self.caching
	}

	/// Get cache time-to-live
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_cache_ttl(300);
	/// assert_eq!(proxy.cache_ttl(), Some(300));
	/// ```
	pub fn cache_ttl(&self) -> Option<u64> {
		self.cache_ttl
	}

	/// Check if triggers are enabled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("users", "name")
	///     .with_triggers(true);
	/// assert!(proxy.has_triggers());
	/// ```
	pub fn has_triggers(&self) -> bool {
		self.triggers
	}

	/// Get trigger events
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("logs", "action")
	///     .with_trigger_events(&["INSERT", "UPDATE"]);
	/// assert_eq!(proxy.trigger_events().len(), 2);
	/// ```
	pub fn trigger_events(&self) -> &[String] {
		&self.trigger_events
	}

	/// Get stored procedure name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("reports", "data")
	///     .with_stored_procedure("generate_report");
	/// assert_eq!(proxy.stored_procedure(), Some("generate_report"));
	/// ```
	pub fn stored_procedure(&self) -> Option<&str> {
		self.stored_procedure.as_deref()
	}

	/// Get stored procedure parameters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("reports", "data")
	///     .with_procedure_params(&[("year", "2024")]);
	/// assert_eq!(proxy.procedure_params().len(), 1);
	/// ```
	pub fn procedure_params(&self) -> &[(String, String)] {
		&self.procedure_params
	}

	/// Get target database name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("users", "name")
	///     .with_database("primary");
	/// assert_eq!(proxy.database(), Some("primary"));
	/// ```
	pub fn database(&self) -> Option<&str> {
		self.database.as_deref()
	}

	/// Get fallback database name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("users", "name")
	///     .with_fallback_database("replica");
	/// assert_eq!(proxy.fallback_database(), Some("replica"));
	/// ```
	pub fn fallback_database(&self) -> Option<&str> {
		self.fallback_database.as_deref()
	}

	/// Check if async loading is enabled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("posts", "title")
	///     .with_async_loading(true);
	/// assert!(proxy.is_async_loading());
	/// ```
	pub fn is_async_loading(&self) -> bool {
		self.async_loading
	}

	/// Check if concurrent access is supported
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::CollectionProxy;
	///
	/// let proxy = CollectionProxy::new("sessions", "data")
	///     .with_concurrent_access(true);
	/// assert!(proxy.supports_concurrent_access());
	/// ```
	pub fn supports_concurrent_access(&self) -> bool {
		self.concurrent_access
	}
}

// Manual Debug implementation for CollectionProxy
impl std::fmt::Debug for CollectionProxy {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CollectionProxy")
			.field("relationship", &self.relationship)
			.field("attribute", &self.attribute)
			.field("unique", &self.unique)
			.field("deduplicate", &self.deduplicate)
			.field("loading_strategy", &self.loading_strategy)
			.field("factory", &self.factory.as_ref().map(|_| "Some(<factory>)"))
			.finish()
	}
}

// Manual Serialize implementation (factory and loading_strategy are not serializable)
impl Serialize for CollectionProxy {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeStruct;
		let mut state = serializer.serialize_struct("CollectionProxy", 3)?;
		state.serialize_field("relationship", &self.relationship)?;
		state.serialize_field("attribute", &self.attribute)?;
		state.serialize_field("unique", &self.unique)?;
		// loading_strategy is not serialized (not Serialize)
		state.end()
	}
}

// Manual Deserialize implementation (factory and loading_strategy are restored as None)
impl<'de> Deserialize<'de> for CollectionProxy {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "snake_case")]
		enum Field {
			Relationship,
			Attribute,
			Unique,
		}

		struct CollectionProxyVisitor;

		impl<'de> serde::de::Visitor<'de> for CollectionProxyVisitor {
			type Value = CollectionProxy;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("struct CollectionProxy")
			}

			fn visit_map<V>(self, mut map: V) -> Result<CollectionProxy, V::Error>
			where
				V: serde::de::MapAccess<'de>,
			{
				let mut relationship = None;
				let mut attribute = None;
				let mut unique = None;

				while let Some(key) = map.next_key()? {
					match key {
						Field::Relationship => {
							if relationship.is_some() {
								return Err(serde::de::Error::duplicate_field("relationship"));
							}
							relationship = Some(map.next_value()?);
						}
						Field::Attribute => {
							if attribute.is_some() {
								return Err(serde::de::Error::duplicate_field("attribute"));
							}
							attribute = Some(map.next_value()?);
						}
						Field::Unique => {
							if unique.is_some() {
								return Err(serde::de::Error::duplicate_field("unique"));
							}
							unique = Some(map.next_value()?);
						}
					}
				}

				let relationship =
					relationship.ok_or_else(|| serde::de::Error::missing_field("relationship"))?;
				let attribute =
					attribute.ok_or_else(|| serde::de::Error::missing_field("attribute"))?;
				let unique = unique.ok_or_else(|| serde::de::Error::missing_field("unique"))?;

				Ok(CollectionProxy {
					relationship,
					attribute,
					unique,
					deduplicate: false,     // Default value
					loading_strategy: None, // Cannot be deserialized
					factory: None,          // Cannot be deserialized
					caching: false,
					cache_ttl: None,
					streaming: false,
					triggers: false,
					trigger_events: Vec::new(),
					stored_procedure: None,
					procedure_params: Vec::new(),
					database: None,
					fallback_database: None,
					async_loading: false,
					concurrent_access: false,
				})
			}
		}

		const FIELDS: &[&str] = &["relationship", "attribute", "unique"];
		deserializer.deserialize_struct("CollectionProxy", FIELDS, CollectionProxyVisitor)
	}
}
