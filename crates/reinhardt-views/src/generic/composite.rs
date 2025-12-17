//! Composite API Views that combine multiple operations

use async_trait::async_trait;
use hyper::Method;
use reinhardt_core::exception::{Error, Result};
use reinhardt_core::http::{Request, Response};
use reinhardt_db::orm::{Filter, FilterOperator, FilterValue, Manager, Model, QuerySet};
use reinhardt_serializers::Serializer;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::core::View;

// Placeholder types for pagination, filtering, and validation configuration.
// These can be replaced with reinhardt_viewsets::{PaginationConfig, FilterConfig}
// when full integration with the viewsets pagination/filtering system is needed.
type PaginationConfig = ();
type FilterConfig = ();
type ValidationConfig = ();

/// ListCreateAPIView combines list and create operations
///
/// This view allows clients to:
/// - GET: List all objects (with pagination, filtering, ordering)
/// - POST: Create a new object
///
/// Similar to Django REST Framework's ListCreateAPIView.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_views::ListCreateAPIView;
/// use reinhardt_db::orm::Model;
/// use reinhardt_serializers::JsonSerializer;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct Article {
///     id: Option<i64>,
///     title: String,
///     content: String,
/// }
///
/// impl Model for Article {
///     type PrimaryKey = i64;
///     fn table_name() -> &'static str { "articles" }
///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// }
///
/// let view = ListCreateAPIView::<Article, JsonSerializer<Article>>::new()
///     .with_paginate_by(10);
/// ```
pub struct ListCreateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
	S: Serializer<Input = M, Output = String> + Send + Sync,
{
	queryset: Option<QuerySet<M>>,
	pagination_config: Option<PaginationConfig>,
	#[allow(dead_code)] // TODO: Will be used when filter implementation is complete
	filter_config: Option<FilterConfig>,
	ordering: Option<Vec<String>>,
	#[allow(dead_code)] // TODO: Will be used when validation implementation is complete
	validation_config: Option<ValidationConfig>,
	_serializer: PhantomData<S>,
}

impl<M, S> ListCreateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self {
			queryset: None,
			pagination_config: None,
			filter_config: None,
			ordering: None,
			validation_config: None,
			_serializer: PhantomData,
		}
	}

	pub fn with_queryset(mut self, queryset: QuerySet<M>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	pub fn with_paginate_by(mut self, _page_size: usize) -> Self {
		// TODO: Implement pagination configuration
		self.pagination_config = Some(());
		self
	}

	pub fn with_ordering(mut self, ordering: Vec<String>) -> Self {
		self.ordering = Some(ordering);
		self
	}

	/// Gets the queryset, creating a default one if not set
	fn get_queryset(&self) -> QuerySet<M> {
		self.queryset.clone().unwrap_or_default()
	}

	/// Gets the objects to display
	async fn get_objects(&self, _request: &Request) -> Result<Vec<M>> {
		let mut queryset = self.get_queryset();

		// Apply ordering if configured
		if let Some(ref ordering) = self.ordering {
			let order_fields: Vec<&str> = ordering.iter().map(|s| s.as_str()).collect();
			queryset = queryset.order_by(&order_fields);
		}

		// TODO: Apply filtering based on request parameters

		// TODO: Apply pagination based on request parameters

		// For now, return all objects (pagination will be added later)
		queryset.all().await.map_err(|e| Error::Http(e.to_string()))
	}
}

impl<M, S> Default for ListCreateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<M, S> View for ListCreateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static + Default,
{
	async fn dispatch(&self, request: Request) -> Result<Response> {
		match request.method {
			Method::GET | Method::HEAD => {
				// List logic (from ListAPIView pattern)
				let objects = self.get_objects(&request).await?;

				// Serialize the objects
				let serializer = S::default();
				let serialized = objects
					.iter()
					.map(|obj| {
						serializer
							.serialize(obj)
							.map_err(|e| Error::Http(e.to_string()))
					})
					.collect::<Result<Vec<_>>>()?;

				// Build response with pagination metadata
				let response_body = if self.pagination_config.is_some() {
					serde_json::json!({
						"count": serialized.len(),
						"results": serialized.iter()
							.filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
							.collect::<Vec<_>>()
					})
				} else {
					serde_json::json!(
						serialized
							.iter()
							.filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
							.collect::<Vec<_>>()
					)
				};

				Response::ok().with_json(&response_body)
			}
			Method::POST => {
				// Create logic
				let data: M = request
					.json()
					.map_err(|e| Error::Http(format!("Invalid request body: {}", e)))?;

				let queryset = self.get_queryset();
				let created = queryset
					.create(data)
					.await
					.map_err(|e| Error::Http(format!("Failed to create: {}", e)))?;

				// Serialize the created object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&created)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse to JSON value for response
				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::created().with_json(&json_value)
			}
			_ => Err(Error::Http("Method not allowed".to_string())),
		}
	}

	fn allowed_methods(&self) -> Vec<&'static str> {
		vec!["GET", "HEAD", "POST", "OPTIONS"]
	}
}

/// RetrieveUpdateAPIView combines retrieve and update operations
///
/// This view allows clients to:
/// - GET: Retrieve a single object
/// - PUT/PATCH: Update an existing object
///
/// Similar to Django REST Framework's RetrieveUpdateAPIView.
pub struct RetrieveUpdateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
	S: Serializer<Input = M, Output = String> + Send + Sync,
{
	queryset: Option<QuerySet<M>>,
	lookup_field: String,
	_serializer: PhantomData<S>,
}

impl<M, S> RetrieveUpdateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self {
			queryset: None,
			lookup_field: "pk".to_string(),
			_serializer: PhantomData,
		}
	}

	pub fn with_queryset(mut self, queryset: QuerySet<M>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	pub fn with_lookup_field(mut self, field: String) -> Self {
		self.lookup_field = field;
		self
	}

	/// Gets the queryset, creating a default one if not set
	fn get_queryset(&self) -> QuerySet<M> {
		self.queryset.clone().unwrap_or_default()
	}

	/// Gets a single object by lookup field value from request path params
	async fn get_object(&self, request: &Request) -> Result<M>
	where
		M: serde::de::DeserializeOwned,
	{
		let lookup_value = request.path_params.get(&self.lookup_field).ok_or_else(|| {
			Error::Http(format!(
				"Missing lookup field '{}' in path parameters",
				self.lookup_field
			))
		})?;

		let filter = Filter::new(
			self.lookup_field.clone(),
			FilterOperator::Eq,
			FilterValue::String(lookup_value.clone()),
		);

		self.get_queryset()
			.filter(filter)
			.get()
			.await
			.map_err(|e| Error::Http(format!("Object not found: {}", e)))
	}
}

impl<M, S> Default for RetrieveUpdateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<M, S> View for RetrieveUpdateAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static + Default,
{
	async fn dispatch(&self, request: Request) -> Result<Response> {
		match request.method {
			Method::GET | Method::HEAD => {
				// Retrieve logic
				let object = self.get_object(&request).await?;

				// Serialize the object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&object)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse to JSON value for response
				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::PUT => {
				// Full update logic - replace all fields
				let mut object = self.get_object(&request).await?;
				let update_data: M = request
					.json()
					.map_err(|e| Error::Http(format!("Invalid request body: {}", e)))?;

				// Get the primary key from existing object to preserve identity
				let pk = object
					.primary_key()
					.cloned()
					.ok_or_else(|| Error::Http("Object has no primary key".to_string()))?;

				// Replace object with update data but keep the same PK
				object = update_data;
				object.set_primary_key(pk);

				// Update using Manager
				let manager = Manager::<M>::new();
				let updated = manager
					.update(&object)
					.await
					.map_err(|e| Error::Http(format!("Failed to update: {}", e)))?;

				// Serialize the updated object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&updated)
					.map_err(|e| Error::Http(e.to_string()))?;

				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::PATCH => {
				// Partial update logic - only update provided fields
				let object = self.get_object(&request).await?;

				// Serialize current object to JSON
				let serializer = S::default();
				let current_json = serializer
					.serialize(&object)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse current object as JSON value
				let mut current: serde_json::Value = serde_json::from_str(&current_json)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				// Parse patch data
				let patch_data: serde_json::Value = request
					.json()
					.map_err(|e| Error::Http(format!("Invalid request body: {}", e)))?;

				// Merge patch data into current object
				if let (Some(current_obj), Some(patch_obj)) =
					(current.as_object_mut(), patch_data.as_object())
				{
					for (key, value) in patch_obj {
						current_obj.insert(key.clone(), value.clone());
					}
				}

				// Deserialize merged object back to model
				let merged: M = serde_json::from_value(current)
					.map_err(|e| Error::Http(format!("Failed to merge patch: {}", e)))?;

				// Update using Manager
				let manager = Manager::<M>::new();
				let updated = manager
					.update(&merged)
					.await
					.map_err(|e| Error::Http(format!("Failed to update: {}", e)))?;

				// Serialize the updated object
				let serialized = serializer
					.serialize(&updated)
					.map_err(|e| Error::Http(e.to_string()))?;

				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			_ => Err(Error::Http("Method not allowed".to_string())),
		}
	}

	fn allowed_methods(&self) -> Vec<&'static str> {
		vec!["GET", "HEAD", "PUT", "PATCH", "OPTIONS"]
	}
}

/// RetrieveDestroyAPIView combines retrieve and destroy operations
///
/// This view allows clients to:
/// - GET: Retrieve a single object
/// - DELETE: Delete an existing object
///
/// Similar to Django REST Framework's RetrieveDestroyAPIView.
pub struct RetrieveDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
	S: Serializer<Input = M, Output = String> + Send + Sync,
{
	queryset: Option<QuerySet<M>>,
	lookup_field: String,
	_serializer: PhantomData<S>,
}

impl<M, S> RetrieveDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self {
			queryset: None,
			lookup_field: "pk".to_string(),
			_serializer: PhantomData,
		}
	}

	pub fn with_queryset(mut self, queryset: QuerySet<M>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	pub fn with_lookup_field(mut self, field: String) -> Self {
		self.lookup_field = field;
		self
	}

	/// Gets the queryset, creating a default one if not set
	fn get_queryset(&self) -> QuerySet<M> {
		self.queryset.clone().unwrap_or_default()
	}

	/// Gets a single object by lookup field value from request path params
	async fn get_object(&self, request: &Request) -> Result<M>
	where
		M: serde::de::DeserializeOwned,
	{
		let lookup_value = request.path_params.get(&self.lookup_field).ok_or_else(|| {
			Error::Http(format!(
				"Missing lookup field '{}' in path parameters",
				self.lookup_field
			))
		})?;

		let filter = Filter::new(
			self.lookup_field.clone(),
			FilterOperator::Eq,
			FilterValue::String(lookup_value.clone()),
		);

		self.get_queryset()
			.filter(filter)
			.get()
			.await
			.map_err(|e| Error::Http(format!("Object not found: {}", e)))
	}
}

impl<M, S> Default for RetrieveDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<M, S> View for RetrieveDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static + Default,
{
	async fn dispatch(&self, request: Request) -> Result<Response> {
		match request.method {
			Method::GET | Method::HEAD => {
				// Retrieve logic
				let object = self.get_object(&request).await?;

				// Serialize the object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&object)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse to JSON value for response
				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::DELETE => {
				// Destroy logic - get object first to ensure it exists
				let object = self.get_object(&request).await?;

				// Get the primary key for deletion
				let pk = object
					.primary_key()
					.cloned()
					.ok_or_else(|| Error::Http("Object has no primary key".to_string()))?;

				// Delete using Manager
				let manager = Manager::<M>::new();
				manager
					.delete(pk)
					.await
					.map_err(|e| Error::Http(format!("Failed to delete: {}", e)))?;

				// Return 204 No Content
				Ok(Response::no_content())
			}
			_ => Err(Error::Http("Method not allowed".to_string())),
		}
	}

	fn allowed_methods(&self) -> Vec<&'static str> {
		vec!["GET", "HEAD", "DELETE", "OPTIONS"]
	}
}

/// RetrieveUpdateDestroyAPIView combines retrieve, update, and destroy operations
///
/// This view allows clients to:
/// - GET: Retrieve a single object
/// - PUT/PATCH: Update an existing object
/// - DELETE: Delete an existing object
///
/// Similar to Django REST Framework's RetrieveUpdateDestroyAPIView.
pub struct RetrieveUpdateDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
	S: Serializer<Input = M, Output = String> + Send + Sync,
{
	queryset: Option<QuerySet<M>>,
	lookup_field: String,
	_serializer: PhantomData<S>,
}

impl<M, S> RetrieveUpdateDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self {
			queryset: None,
			lookup_field: "pk".to_string(),
			_serializer: PhantomData,
		}
	}

	pub fn with_queryset(mut self, queryset: QuerySet<M>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	pub fn with_lookup_field(mut self, field: String) -> Self {
		self.lookup_field = field;
		self
	}

	/// Gets the queryset, creating a default one if not set
	fn get_queryset(&self) -> QuerySet<M> {
		self.queryset.clone().unwrap_or_default()
	}

	/// Gets a single object by lookup field value from request path params
	async fn get_object(&self, request: &Request) -> Result<M>
	where
		M: serde::de::DeserializeOwned,
	{
		let lookup_value = request.path_params.get(&self.lookup_field).ok_or_else(|| {
			Error::Http(format!(
				"Missing lookup field '{}' in path parameters",
				self.lookup_field
			))
		})?;

		let filter = Filter::new(
			self.lookup_field.clone(),
			FilterOperator::Eq,
			FilterValue::String(lookup_value.clone()),
		);

		self.get_queryset()
			.filter(filter)
			.get()
			.await
			.map_err(|e| Error::Http(format!("Object not found: {}", e)))
	}
}

impl<M, S> Default for RetrieveUpdateDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<M, S> View for RetrieveUpdateDestroyAPIView<M, S>
where
	M: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
	S: Serializer<Input = M, Output = String> + Send + Sync + 'static + Default,
{
	async fn dispatch(&self, request: Request) -> Result<Response> {
		match request.method {
			Method::GET | Method::HEAD => {
				// Retrieve logic
				let object = self.get_object(&request).await?;

				// Serialize the object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&object)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse to JSON value for response
				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::PUT => {
				// Full update logic - replace all fields
				let mut object = self.get_object(&request).await?;
				let update_data: M = request
					.json()
					.map_err(|e| Error::Http(format!("Invalid request body: {}", e)))?;

				// Get the primary key from existing object to preserve identity
				let pk = object
					.primary_key()
					.cloned()
					.ok_or_else(|| Error::Http("Object has no primary key".to_string()))?;

				// Replace object with update data but keep the same PK
				object = update_data;
				object.set_primary_key(pk);

				// Update using Manager
				let manager = Manager::<M>::new();
				let updated = manager
					.update(&object)
					.await
					.map_err(|e| Error::Http(format!("Failed to update: {}", e)))?;

				// Serialize the updated object
				let serializer = S::default();
				let serialized = serializer
					.serialize(&updated)
					.map_err(|e| Error::Http(e.to_string()))?;

				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::PATCH => {
				// Partial update logic - only update provided fields
				let object = self.get_object(&request).await?;

				// Serialize current object to JSON
				let serializer = S::default();
				let current_json = serializer
					.serialize(&object)
					.map_err(|e| Error::Http(e.to_string()))?;

				// Parse current object as JSON value
				let mut current: serde_json::Value = serde_json::from_str(&current_json)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				// Parse patch data
				let patch_data: serde_json::Value = request
					.json()
					.map_err(|e| Error::Http(format!("Invalid request body: {}", e)))?;

				// Merge patch data into current object
				if let (Some(current_obj), Some(patch_obj)) =
					(current.as_object_mut(), patch_data.as_object())
				{
					for (key, value) in patch_obj {
						current_obj.insert(key.clone(), value.clone());
					}
				}

				// Deserialize merged object back to model
				let merged: M = serde_json::from_value(current)
					.map_err(|e| Error::Http(format!("Failed to merge patch: {}", e)))?;

				// Update using Manager
				let manager = Manager::<M>::new();
				let updated = manager
					.update(&merged)
					.await
					.map_err(|e| Error::Http(format!("Failed to update: {}", e)))?;

				// Serialize the updated object
				let serialized = serializer
					.serialize(&updated)
					.map_err(|e| Error::Http(e.to_string()))?;

				let json_value: serde_json::Value = serde_json::from_str(&serialized)
					.map_err(|e| Error::Http(format!("Serialization error: {}", e)))?;

				Response::ok().with_json(&json_value)
			}
			Method::DELETE => {
				// Destroy logic - get object first to ensure it exists
				let object = self.get_object(&request).await?;

				// Get the primary key for deletion
				let pk = object
					.primary_key()
					.cloned()
					.ok_or_else(|| Error::Http("Object has no primary key".to_string()))?;

				// Delete using Manager
				let manager = Manager::<M>::new();
				manager
					.delete(pk)
					.await
					.map_err(|e| Error::Http(format!("Failed to delete: {}", e)))?;

				// Return 204 No Content
				Ok(Response::no_content())
			}
			_ => Err(Error::Http("Method not allowed".to_string())),
		}
	}

	fn allowed_methods(&self) -> Vec<&'static str> {
		vec!["GET", "HEAD", "PUT", "PATCH", "DELETE", "OPTIONS"]
	}
}
