//! ORM integration for relation fields
//!
//! This module provides ORM-backed implementations of relation fields:
//! - `PrimaryKeyRelatedField` with database lookups
//! - `SlugRelatedField` with slug-based queries
//! - Query optimization with select_related/prefetch_related

use super::{SerializerError, ValidatorError};
use async_trait::async_trait;
use reinhardt_db::orm::{Model, query::*};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

// Type wrappers to simplify complex function pointer types

/// Type alias for async many-to-many relation resolver trait object
type ManyResolverFn<T, RelatedItem> = Arc<
	dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<Vec<RelatedItem>, String>> + Send>>
		+ Send
		+ Sync,
>;

/// Async many-to-many relation resolver function
pub struct ManyRelationResolverFn<T, RelatedItem> {
	inner: ManyResolverFn<T, RelatedItem>,
	_phantom: PhantomData<RelatedItem>,
}

impl<T, RelatedItem> ManyRelationResolverFn<T, RelatedItem> {
	/// Create a new many-to-many relation resolver
	pub fn new<F>(func: F) -> Self
	where
		F: Fn(&T) -> Pin<Box<dyn Future<Output = Result<Vec<RelatedItem>, String>> + Send>>
			+ Send
			+ Sync
			+ 'static,
	{
		Self {
			inner: Arc::new(func),
			_phantom: PhantomData,
		}
	}
}

impl<T, RelatedItem> std::ops::Deref for ManyRelationResolverFn<T, RelatedItem> {
	type Target = dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<Vec<RelatedItem>, String>> + Send>>
		+ Send
		+ Sync;

	fn deref(&self) -> &Self::Target {
		&*self.inner
	}
}

impl<T, RelatedItem> Clone for ManyRelationResolverFn<T, RelatedItem> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_phantom: PhantomData,
		}
	}
}

/// Type alias for async one-to-one/foreign-key relation resolver trait object
type SingleResolverFn<T, RelatedItem> = Arc<
	dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<RelatedItem, String>> + Send>> + Send + Sync,
>;

/// Async one-to-one/foreign-key relation resolver function
pub struct SingleRelationResolverFn<T, RelatedItem> {
	inner: SingleResolverFn<T, RelatedItem>,
	_phantom: PhantomData<RelatedItem>,
}

impl<T, RelatedItem> SingleRelationResolverFn<T, RelatedItem> {
	/// Create a new single relation resolver
	pub fn new<F>(func: F) -> Self
	where
		F: Fn(&T) -> Pin<Box<dyn Future<Output = Result<RelatedItem, String>> + Send>>
			+ Send
			+ Sync
			+ 'static,
	{
		Self {
			inner: Arc::new(func),
			_phantom: PhantomData,
		}
	}
}

impl<T, RelatedItem> std::ops::Deref for SingleRelationResolverFn<T, RelatedItem> {
	type Target = dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<RelatedItem, String>> + Send>>
		+ Send
		+ Sync;

	fn deref(&self) -> &Self::Target {
		&*self.inner
	}
}

impl<T, RelatedItem> Clone for SingleRelationResolverFn<T, RelatedItem> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_phantom: PhantomData,
		}
	}
}

/// Primary key related field with ORM query support
///
/// Represents a relationship using the related object's primary key.
/// Provides database lookup and validation.
///
/// # Examples
///
/// ```rust,no_run,ignore
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_rest::serializers::relation_fields_orm::PrimaryKeyRelatedFieldORM;
///
/// // Field that references User by ID
/// let field = PrimaryKeyRelatedFieldORM::<User>::new();
///
/// // Verify that user exists (database lookup)
/// field.validate_exists(123).await?;
///
/// // Get the user instance (database query)
/// let user = field.get_instance(123).await?;
///
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PrimaryKeyRelatedFieldORM<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
{
	_phantom: PhantomData<T>,
	/// Whether to allow null values
	pub allow_null: bool,
	/// Custom queryset for filtering
	pub queryset_filter: Option<Filter>,
}

impl<T> PrimaryKeyRelatedFieldORM<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
{
	/// Create a new primary key related field
	///
	/// # Examples
	///
	/// ```ignore
	/// let field = PrimaryKeyRelatedFieldORM::<User>::new();
	/// // Verify the field is created with default settings
	/// let _: PrimaryKeyRelatedFieldORM<User> = field;
	/// ```
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
			allow_null: false,
			queryset_filter: None,
		}
	}

	/// Allow null values
	pub fn with_allow_null(mut self, allow_null: bool) -> Self {
		self.allow_null = allow_null;
		self
	}

	/// Add queryset filter
	///
	/// Restricts lookups to instances matching the filter.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue};
	///
	/// let field = PrimaryKeyRelatedFieldORM::<User>::new()
	///     .with_queryset_filter(Filter::new(
	///         "is_active".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ));
	/// // Verify the field is configured with the filter
	/// let _: PrimaryKeyRelatedFieldORM<User> = field;
	/// ```
	pub fn with_queryset_filter(mut self, filter: Filter) -> Self {
		self.queryset_filter = Some(filter);
		self
	}

	/// Validate that an instance with the given primary key exists
	///
	/// # Errors
	///
	/// Returns `SerializerError` if:
	/// - Instance not found
	/// - Instance doesn't match queryset filter
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify existence check runs without error (requires database)
	/// field.validate_exists(&123).await?;
	/// ```
	pub async fn validate_exists(&self, pk: &T::PrimaryKey) -> Result<(), SerializerError>
	where
		T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
		T::PrimaryKey: std::fmt::Display + Clone + Send + Sync,
	{
		use reinhardt_db::orm::QuerySet;

		let mut queryset = QuerySet::<T>::new();

		// Filter by primary key
		let pk_field = T::primary_key_field();
		let filter = Filter::new(
			pk_field.to_string(),
			FilterOperator::Eq,
			FilterValue::String(pk.to_string()),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Count results
		let count = queryset.count().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to validate existence: {}", e),
		})?;

		if count == 0 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: format!("Instance with pk {} does not exist", pk),
			}));
		}

		Ok(())
	}

	/// Get the related instance by primary key
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify instance is retrieved correctly (requires database)
	/// let user = field.get_instance(&123).await?;
	/// assert_eq!(user.id, Some(123));
	/// ```
	pub async fn get_instance(&self, pk: &T::PrimaryKey) -> Result<T, SerializerError>
	where
		T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
		T::PrimaryKey: std::fmt::Display + Clone + Send + Sync,
	{
		use reinhardt_db::orm::QuerySet;

		let mut queryset = QuerySet::<T>::new();

		// Filter by primary key
		let pk_field = T::primary_key_field();
		let filter = Filter::new(
			pk_field.to_string(),
			FilterOperator::Eq,
			FilterValue::String(pk.to_string()),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Get first result
		let instance = queryset
			.first()
			.await
			.map_err(|e| SerializerError::Other {
				message: format!("Failed to fetch instance: {}", e),
			})?
			.ok_or_else(|| {
				SerializerError::Validation(ValidatorError::Custom {
					message: format!("Instance with pk {} not found", pk),
				})
			})?;

		Ok(instance)
	}

	/// Get multiple instances by primary keys (batch lookup)
	///
	/// More efficient than calling `get_instance` multiple times.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify batch retrieval works (requires database)
	/// let users = field.get_instances(vec![1, 2, 3]).await?;
	/// assert_eq!(users.len(), 3);
	/// ```
	pub async fn get_instances(&self, pks: Vec<T::PrimaryKey>) -> Result<Vec<T>, SerializerError>
	where
		T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
		T::PrimaryKey: std::fmt::Display + Clone + Send + Sync,
	{
		use reinhardt_db::orm::QuerySet;

		if pks.is_empty() {
			return Ok(Vec::new());
		}

		let mut queryset = QuerySet::<T>::new();

		// Filter by primary key IN (pks)
		let pk_field = T::primary_key_field();
		let pk_values: Vec<String> = pks.iter().map(|pk| pk.to_string()).collect();

		let filter = Filter::new(
			pk_field.to_string(),
			FilterOperator::In,
			FilterValue::Array(pk_values),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Get all results
		let instances = queryset.all().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to fetch instances: {}", e),
		})?;

		Ok(instances)
	}
}

impl<T> Default for PrimaryKeyRelatedFieldORM<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
{
	fn default() -> Self {
		Self::new()
	}
}

/// Slug related field with ORM query support
///
/// Represents a relationship using a slug field (e.g., username, slug).
/// Provides database lookup by slug value.
///
/// # Examples
///
/// ```rust,no_run,ignore
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_rest::serializers::relation_fields_orm::SlugRelatedFieldORM;
///
/// // Field that references User by username
/// let field = SlugRelatedFieldORM::<User>::new("username");
///
/// // Verify that user exists (database lookup by slug)
/// field.validate_exists("alice").await?;
///
/// // Get the user instance (database query by slug)
/// let user = field.get_instance("alice").await?;
/// assert_eq!(user.username, "alice");
///
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SlugRelatedFieldORM<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
{
	_phantom: PhantomData<T>,
	/// The slug field name to query by
	pub slug_field: String,
	/// Whether to allow null values
	pub allow_null: bool,
	/// Custom queryset filter
	pub queryset_filter: Option<Filter>,
}

impl<T> SlugRelatedFieldORM<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync,
{
	/// Create a new slug related field
	///
	/// # Examples
	///
	/// ```ignore
	/// let field = SlugRelatedFieldORM::<User>::new("username");
	/// // Verify the field is created with the slug field
	/// let _: SlugRelatedFieldORM<User> = field;
	/// ```
	pub fn new(slug_field: impl Into<String>) -> Self {
		Self {
			_phantom: PhantomData,
			slug_field: slug_field.into(),
			allow_null: false,
			queryset_filter: None,
		}
	}

	/// Allow null values
	pub fn with_allow_null(mut self, allow_null: bool) -> Self {
		self.allow_null = allow_null;
		self
	}

	/// Add queryset filter
	pub fn with_queryset_filter(mut self, filter: Filter) -> Self {
		self.queryset_filter = Some(filter);
		self
	}

	/// Validate that an instance with the given slug exists
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify slug existence check runs (requires database)
	/// field.validate_exists("alice").await?;
	/// ```
	pub async fn validate_exists(&self, slug: &str) -> Result<(), SerializerError> {
		use reinhardt_db::orm::QuerySet;

		let mut queryset = QuerySet::<T>::new();

		// Filter by slug field
		let filter = Filter::new(
			self.slug_field.clone(),
			FilterOperator::Eq,
			FilterValue::String(slug.to_string()),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Count results
		let count = queryset.count().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to validate existence: {}", e),
		})?;

		if count == 0 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: format!(
					"Instance with {} '{}' does not exist",
					self.slug_field, slug
				),
			}));
		}

		Ok(())
	}

	/// Get the related instance by slug
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify instance retrieval by slug (requires database)
	/// let user = field.get_instance("alice").await?;
	/// assert_eq!(user.username, "alice");
	/// ```
	pub async fn get_instance(&self, slug: &str) -> Result<T, SerializerError> {
		use reinhardt_db::orm::QuerySet;

		let mut queryset = QuerySet::<T>::new();

		// Filter by slug field
		let filter = Filter::new(
			self.slug_field.clone(),
			FilterOperator::Eq,
			FilterValue::String(slug.to_string()),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Get first result
		let instance = queryset
			.first()
			.await
			.map_err(|e| SerializerError::Other {
				message: format!("Failed to fetch instance: {}", e),
			})?
			.ok_or_else(|| {
				SerializerError::Validation(ValidatorError::Custom {
					message: format!("Instance with {} '{}' not found", self.slug_field, slug),
				})
			})?;

		Ok(instance)
	}

	/// Get multiple instances by slugs (batch lookup)
	///
	/// # Examples
	///
	/// ```ignore
	/// // Verify batch retrieval by slug (requires database)
	/// let users = field.get_instances(vec!["alice".to_string(), "bob".to_string()]).await?;
	/// assert_eq!(users.len(), 2);
	/// ```
	pub async fn get_instances(&self, slugs: Vec<String>) -> Result<Vec<T>, SerializerError> {
		use reinhardt_db::orm::QuerySet;

		if slugs.is_empty() {
			return Ok(Vec::new());
		}

		let mut queryset = QuerySet::<T>::new();

		// Filter by slug field IN (slugs)
		let slug_values: Vec<String> = slugs.to_vec();

		let filter = Filter::new(
			self.slug_field.clone(),
			FilterOperator::In,
			FilterValue::Array(slug_values),
		);
		queryset = queryset.filter(filter);

		// Apply additional filter if present
		if let Some(ref custom_filter) = self.queryset_filter {
			queryset = queryset.filter(custom_filter.clone());
		}

		// Get all results
		let instances = queryset.all().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to fetch instances: {}", e),
		})?;

		Ok(instances)
	}
}

/// Query optimization manager for relation fields
///
/// Manages select_related and prefetch_related optimizations
/// for efficient loading of related objects.
///
/// # Examples
///
/// ```rust,no_run,ignore
/// use reinhardt_rest::serializers::relation_fields_orm::QueryOptimizer;
///
/// let optimizer = QueryOptimizer::new()
///     .with_select_related(vec!["author", "category"])
///     .with_prefetch_related(vec!["comments", "tags"]);
///
/// // Verify optimizer is configured correctly
/// let _: QueryOptimizer = optimizer.clone();
/// let queryset = optimizer.apply(QuerySet::<Post>::new());
/// let posts = queryset.all();
/// ```
#[derive(Debug, Clone, Default)]
pub struct QueryOptimizer {
	/// Fields to load via JOIN (select_related)
	pub select_related: Vec<String>,
	/// Fields to load via separate queries (prefetch_related)
	pub prefetch_related: Vec<String>,
}

impl QueryOptimizer {
	/// Create a new query optimizer
	pub fn new() -> Self {
		Self {
			select_related: Vec::new(),
			prefetch_related: Vec::new(),
		}
	}

	/// Add select_related fields
	///
	/// Loads related objects using JOIN queries.
	/// Best for ForeignKey and OneToOne relationships.
	pub fn with_select_related(mut self, fields: Vec<String>) -> Self {
		self.select_related = fields;
		self
	}

	/// Add prefetch_related fields
	///
	/// Loads related objects using separate queries.
	/// Best for ManyToMany and reverse ForeignKey relationships.
	pub fn with_prefetch_related(mut self, fields: Vec<String>) -> Self {
		self.prefetch_related = fields;
		self
	}

	/// Apply optimizations to a QuerySet
	pub fn apply<T>(&self, mut queryset: QuerySet<T>) -> QuerySet<T>
	where
		T: Model,
	{
		if !self.select_related.is_empty() {
			let fields: Vec<&str> = self.select_related.iter().map(|s| s.as_str()).collect();
			queryset = queryset.select_related(&fields);
		}

		if !self.prefetch_related.is_empty() {
			let fields: Vec<&str> = self.prefetch_related.iter().map(|s| s.as_str()).collect();
			queryset = queryset.prefetch_related(&fields);
		}

		queryset
	}
}

/// Trait for relation fields that support query optimization
#[async_trait]
pub trait OptimizableRelationField {
	/// Get the query optimizer for this field
	fn get_optimizer(&self) -> Option<&QueryOptimizer>;

	/// Set the query optimizer for this field
	fn set_optimizer(&mut self, optimizer: QueryOptimizer);
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_db::orm::{FieldSelector, Model};
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestUser {
		id: Option<i64>,
		username: String,
		email: String,
	}

	#[derive(Debug, Clone)]
	struct TestUserFields;

	impl FieldSelector for TestUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		type Fields = TestUserFields;

		fn table_name() -> &'static str {
			"test_users"
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_pk_related_field_creation() {
		let field = PrimaryKeyRelatedFieldORM::<TestUser>::new();
		assert!(!field.allow_null);
		assert!(field.queryset_filter.is_none());
	}

	#[test]
	fn test_pk_related_field_with_allow_null() {
		let field = PrimaryKeyRelatedFieldORM::<TestUser>::new().with_allow_null(true);
		assert!(field.allow_null);
	}

	#[test]
	fn test_pk_related_field_with_filter() {
		let filter = Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		);
		let field = PrimaryKeyRelatedFieldORM::<TestUser>::new().with_queryset_filter(filter);
		assert!(field.queryset_filter.is_some());
	}

	#[test]
	fn test_slug_related_field_creation() {
		let field = SlugRelatedFieldORM::<TestUser>::new("username");
		assert_eq!(field.slug_field, "username");
		assert!(!field.allow_null);
		assert!(field.queryset_filter.is_none());
	}

	#[test]
	fn test_slug_related_field_with_allow_null() {
		let field = SlugRelatedFieldORM::<TestUser>::new("username").with_allow_null(true);
		assert!(field.allow_null);
	}

	#[test]
	fn test_query_optimizer_creation() {
		let optimizer = QueryOptimizer::new();
		assert!(optimizer.select_related.is_empty());
		assert!(optimizer.prefetch_related.is_empty());
	}

	#[test]
	fn test_query_optimizer_with_select_related() {
		let optimizer = QueryOptimizer::new()
			.with_select_related(vec!["author".to_string(), "category".to_string()]);

		assert_eq!(optimizer.select_related.len(), 2);
		assert!(optimizer.select_related.contains(&"author".to_string()));
		assert!(optimizer.select_related.contains(&"category".to_string()));
	}

	#[test]
	fn test_query_optimizer_with_prefetch_related() {
		let optimizer = QueryOptimizer::new()
			.with_prefetch_related(vec!["comments".to_string(), "tags".to_string()]);

		assert_eq!(optimizer.prefetch_related.len(), 2);
		assert!(optimizer.prefetch_related.contains(&"comments".to_string()));
		assert!(optimizer.prefetch_related.contains(&"tags".to_string()));
	}

	#[test]
	fn test_query_optimizer_apply() {
		let optimizer = QueryOptimizer::new()
			.with_select_related(vec!["author".to_string()])
			.with_prefetch_related(vec!["comments".to_string()]);

		let queryset = QuerySet::<TestUser>::new();
		let optimized = optimizer.apply(queryset);

		// QuerySet methods are called, but we can't easily verify the result
		// without a real database. This test just ensures no panics.
		drop(optimized);
	}
}
