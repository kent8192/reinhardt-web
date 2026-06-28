//! QuerySet-like API for client-side data fetching.
//!
//! This module provides a Django QuerySet-inspired interface for making
//! API calls from WASM applications.

use crate::server_fn::ServerFnError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

/// Filter operation types.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FilterOp {
	/// Exact match (field = value).
	#[default]
	Exact,
	/// Case-insensitive exact match.
	IExact,
	/// Contains substring.
	Contains,
	/// Case-insensitive contains.
	IContains,
	/// Greater than.
	Gt,
	/// Greater than or equal.
	Gte,
	/// Less than.
	Lt,
	/// Less than or equal.
	Lte,
	/// Starts with.
	StartsWith,
	/// Case-insensitive starts with.
	IStartsWith,
	/// Ends with.
	EndsWith,
	/// Case-insensitive ends with.
	IEndsWith,
	/// In list of values.
	In,
	/// Is null check.
	IsNull,
	/// Range (between two values).
	Range,
}

/// A single filter condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
	/// The field name to filter on.
	pub field: String,
	/// The filter operation.
	pub op: FilterOp,
	/// The value to filter with.
	pub value: serde_json::Value,
	/// Whether this is an exclude filter (NOT).
	pub exclude: bool,
}

impl Filter {
	/// Creates a new exact match filter.
	pub fn exact(field: impl Into<String>, value: impl Serialize) -> Self {
		Self {
			field: field.into(),
			op: FilterOp::Exact,
			value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
			exclude: false,
		}
	}

	/// Creates a new filter with a specific operation.
	pub fn with_op(field: impl Into<String>, op: FilterOp, value: impl Serialize) -> Self {
		Self {
			field: field.into(),
			op,
			value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
			exclude: false,
		}
	}

	/// Converts this filter to an exclude filter.
	pub fn negate(mut self) -> Self {
		self.exclude = !self.exclude;
		self
	}

	/// Converts the filter to a query parameter string.
	pub fn to_query_param(&self) -> (String, String) {
		let key = match self.op {
			FilterOp::Exact => self.field.clone(),
			FilterOp::IExact => format!("{}__iexact", self.field),
			FilterOp::Contains => format!("{}__contains", self.field),
			FilterOp::IContains => format!("{}__icontains", self.field),
			FilterOp::Gt => format!("{}__gt", self.field),
			FilterOp::Gte => format!("{}__gte", self.field),
			FilterOp::Lt => format!("{}__lt", self.field),
			FilterOp::Lte => format!("{}__lte", self.field),
			FilterOp::StartsWith => format!("{}__startswith", self.field),
			FilterOp::IStartsWith => format!("{}__istartswith", self.field),
			FilterOp::EndsWith => format!("{}__endswith", self.field),
			FilterOp::IEndsWith => format!("{}__iendswith", self.field),
			FilterOp::In => format!("{}__in", self.field),
			FilterOp::IsNull => format!("{}__isnull", self.field),
			FilterOp::Range => format!("{}__range", self.field),
		};

		let value = match &self.value {
			serde_json::Value::String(s) => s.clone(),
			serde_json::Value::Number(n) => n.to_string(),
			serde_json::Value::Bool(b) => b.to_string(),
			serde_json::Value::Array(arr) => arr
				.iter()
				.map(|v| match v {
					serde_json::Value::String(s) => s.clone(),
					other => other.to_string(),
				})
				.collect::<Vec<_>>()
				.join(","),
			serde_json::Value::Null => "null".to_string(),
			other => other.to_string(),
		};

		(key, value)
	}
}

/// A QuerySet-like builder for API requests.
///
/// This provides a fluent interface similar to Django's QuerySet
/// for building and executing API queries.
#[derive(Debug, Clone)]
pub struct ApiQuerySet<T> {
	/// The API endpoint URL.
	endpoint: String,
	/// Filter conditions.
	filters: Vec<Filter>,
	/// Ordering fields (prefix with '-' for descending).
	ordering: Vec<String>,
	/// Maximum number of results.
	limit: Option<usize>,
	/// Number of results to skip.
	offset: Option<usize>,
	/// Fields to select (for partial responses).
	fields: Vec<String>,
	/// PhantomData for the model type.
	_marker: PhantomData<T>,
}

impl<T> ApiQuerySet<T>
where
	T: Serialize + DeserializeOwned,
{
	/// Creates a new QuerySet for the given endpoint.
	pub fn new(endpoint: impl Into<String>) -> Self {
		Self {
			endpoint: endpoint.into(),
			filters: Vec::new(),
			ordering: Vec::new(),
			limit: None,
			offset: None,
			fields: Vec::new(),
			_marker: PhantomData,
		}
	}

	/// Adds a filter condition (exact match).
	///
	/// # Example
	/// ```ignore
	/// User::objects().filter("is_active", true)
	/// ```
	pub fn filter(mut self, field: impl Into<String>, value: impl Serialize) -> Self {
		self.filters.push(Filter::exact(field, value));
		self
	}

	/// Adds a filter with a specific operation.
	///
	/// # Example
	/// ```ignore
	/// User::objects().filter_op("age", FilterOp::Gte, 18)
	/// ```
	pub fn filter_op(
		mut self,
		field: impl Into<String>,
		op: FilterOp,
		value: impl Serialize,
	) -> Self {
		self.filters.push(Filter::with_op(field, op, value));
		self
	}

	/// Adds an exclude filter (NOT condition).
	///
	/// # Example
	/// ```ignore
	/// User::objects().exclude("status", "banned")
	/// ```
	pub fn exclude(mut self, field: impl Into<String>, value: impl Serialize) -> Self {
		self.filters.push(Filter::exact(field, value).negate());
		self
	}

	/// Sets the ordering for results.
	///
	/// Prefix field names with '-' for descending order.
	///
	/// # Example
	/// ```ignore
	/// User::objects().order_by(&["-created_at", "username"])
	/// ```
	pub fn order_by(mut self, fields: &[&str]) -> Self {
		self.ordering = fields.iter().map(|s| (*s).to_string()).collect();
		self
	}

	/// Limits the number of results.
	///
	/// # Example
	/// ```ignore
	/// User::objects().limit(10)
	/// ```
	pub fn limit(mut self, n: usize) -> Self {
		self.limit = Some(n);
		self
	}

	/// Skips the first N results.
	///
	/// # Example
	/// ```ignore
	/// User::objects().offset(20).limit(10)  // Page 3
	/// ```
	pub fn offset(mut self, n: usize) -> Self {
		self.offset = Some(n);
		self
	}

	/// Selects specific fields for partial responses.
	///
	/// # Example
	/// ```ignore
	/// User::objects().only(&["id", "username"])
	/// ```
	pub fn only(mut self, fields: &[&str]) -> Self {
		self.fields = fields.iter().map(|s| (*s).to_string()).collect();
		self
	}

	/// Returns a clone of this QuerySet with no filters or ordering.
	pub fn all_clone(&self) -> Self {
		Self::new(&self.endpoint)
	}

	/// Builds the query URL with all parameters.
	pub fn build_url(&self) -> String {
		let mut params: Vec<(String, String)> = Vec::new();

		// Add filters
		for filter in &self.filters {
			let (key, value) = filter.to_query_param();
			if filter.exclude {
				params.push((format!("exclude__{}", key), value));
			} else {
				params.push((key, value));
			}
		}

		// Add ordering
		if !self.ordering.is_empty() {
			params.push(("ordering".to_string(), self.ordering.join(",")));
		}

		// Add pagination
		if let Some(limit) = self.limit {
			params.push(("limit".to_string(), limit.to_string()));
		}
		if let Some(offset) = self.offset {
			params.push(("offset".to_string(), offset.to_string()));
		}

		// Add field selection
		if !self.fields.is_empty() {
			params.push(("fields".to_string(), self.fields.join(",")));
		}

		// Build URL
		if params.is_empty() {
			self.endpoint.clone()
		} else {
			let query_string = params
				.iter()
				.map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
				.collect::<Vec<_>>()
				.join("&");
			format!("{}?{}", self.endpoint, query_string)
		}
	}

	#[cfg(wasm)]
	async fn fetch_response(
		&self,
		method: &str,
		url: &str,
		body: Option<&str>,
	) -> Result<crate::fetch::FetchResponse, ServerFnError> {
		use crate::csrf::csrf_headers;
		use crate::fetch;

		let mut headers = Vec::new();
		if body.is_some() {
			headers.push(("Content-Type".to_string(), "application/json".to_string()));
		}
		if let Some((header_name, header_value)) = csrf_headers() {
			headers.push((header_name.to_string(), header_value));
		}

		let response = fetch::request(method, url, body, headers).await?;
		if !response.is_success() {
			return Err(ServerFnError::server(
				response.status(),
				response.into_text(),
			));
		}
		Ok(response)
	}

	#[cfg(wasm)]
	async fn fetch_json<R>(
		&self,
		method: &str,
		url: &str,
		body: Option<&str>,
	) -> Result<R, ServerFnError>
	where
		R: DeserializeOwned,
	{
		self.fetch_response(method, url, body).await?.json()
	}

	/// Fetches all matching results.
	#[cfg(wasm)]
	pub async fn all(&self) -> Result<Vec<T>, ServerFnError> {
		let url = self.build_url();
		self.fetch_json("GET", &url, None).await
	}

	/// Fetches all matching results (non-WASM stub).
	#[cfg(native)]
	pub async fn all(&self) -> Result<Vec<T>, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Fetches the first matching result.
	#[cfg(wasm)]
	pub async fn first(&self) -> Result<Option<T>, ServerFnError>
	where
		T: Clone,
	{
		let mut queryset = self.clone();
		queryset.limit = Some(1);
		let results = queryset.all().await?;
		Ok(results.into_iter().next())
	}

	/// Fetches the first matching result (non-WASM stub).
	#[cfg(native)]
	pub async fn first(&self) -> Result<Option<T>, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Fetches a single result by primary key.
	#[cfg(wasm)]
	pub async fn get(&self, pk: impl std::fmt::Display) -> Result<T, ServerFnError> {
		let url = format!("{}{}/", self.endpoint.trim_end_matches('/'), pk);
		self.fetch_json("GET", &url, None).await
	}

	/// Fetches a single result by primary key (non-WASM stub).
	#[cfg(native)]
	pub async fn get(&self, _pk: impl std::fmt::Display) -> Result<T, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Returns the count of matching results.
	#[cfg(wasm)]
	pub async fn count(&self) -> Result<usize, ServerFnError> {
		let base_url = self.build_url();
		let separator = if base_url.contains('?') { '&' } else { '?' };
		let url = format!("{base_url}{separator}count=true");

		#[derive(Deserialize)]
		struct CountResponse {
			count: usize,
		}

		let result: CountResponse = self.fetch_json("GET", &url, None).await?;
		Ok(result.count)
	}

	/// Returns the count of matching results (non-WASM stub).
	#[cfg(native)]
	pub async fn count(&self) -> Result<usize, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Checks if any matching results exist.
	pub async fn exists(&self) -> Result<bool, ServerFnError>
	where
		Self: Clone,
	{
		let count = self.clone().limit(1).count().await?;
		Ok(count > 0)
	}

	/// Creates a new record.
	#[cfg(wasm)]
	pub async fn create(&self, data: &T) -> Result<T, ServerFnError> {
		let body =
			serde_json::to_string(data).map_err(|e| ServerFnError::Serialization(e.to_string()))?;
		self.fetch_json("POST", &self.endpoint, Some(&body)).await
	}

	/// Creates a new record (non-WASM stub).
	#[cfg(native)]
	pub async fn create(&self, _data: &T) -> Result<T, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Updates an existing record.
	#[cfg(wasm)]
	pub async fn update(&self, pk: impl std::fmt::Display, data: &T) -> Result<T, ServerFnError> {
		let url = format!("{}{}/", self.endpoint.trim_end_matches('/'), pk);
		let body =
			serde_json::to_string(data).map_err(|e| ServerFnError::Serialization(e.to_string()))?;
		self.fetch_json("PUT", &url, Some(&body)).await
	}

	/// Updates an existing record (non-WASM stub).
	#[cfg(native)]
	pub async fn update(&self, _pk: impl std::fmt::Display, _data: &T) -> Result<T, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Partially updates an existing record.
	#[cfg(wasm)]
	pub async fn partial_update(
		&self,
		pk: impl std::fmt::Display,
		data: &serde_json::Value,
	) -> Result<T, ServerFnError> {
		let url = format!("{}{}/", self.endpoint.trim_end_matches('/'), pk);
		let body =
			serde_json::to_string(data).map_err(|e| ServerFnError::Serialization(e.to_string()))?;
		self.fetch_json("PATCH", &url, Some(&body)).await
	}

	/// Partially updates an existing record (non-WASM stub).
	#[cfg(native)]
	pub async fn partial_update(
		&self,
		_pk: impl std::fmt::Display,
		_data: &serde_json::Value,
	) -> Result<T, ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}

	/// Deletes a record by primary key.
	#[cfg(wasm)]
	pub async fn delete(&self, pk: impl std::fmt::Display) -> Result<(), ServerFnError> {
		let url = format!("{}{}/", self.endpoint.trim_end_matches('/'), pk);
		self.fetch_response("DELETE", &url, None).await?;
		Ok(())
	}

	/// Deletes a record by primary key (non-WASM stub).
	#[cfg(native)]
	pub async fn delete(&self, _pk: impl std::fmt::Display) -> Result<(), ServerFnError> {
		Err(ServerFnError::Network(
			"API calls not supported outside WASM".to_string(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_filter_exact() {
		let filter = Filter::exact("name", "test");
		assert_eq!(filter.field, "name");
		assert!(!filter.exclude);
		let (key, value) = filter.to_query_param();
		assert_eq!(key, "name");
		assert_eq!(value, "test");
	}

	#[test]
	fn test_filter_with_op() {
		let filter = Filter::with_op("age", FilterOp::Gte, 18);
		let (key, value) = filter.to_query_param();
		assert_eq!(key, "age__gte");
		assert_eq!(value, "18");
	}

	#[test]
	fn test_filter_negate() {
		let filter = Filter::exact("status", "banned").negate();
		assert!(filter.exclude);
	}

	#[test]
	fn test_queryset_build_url_simple() {
		let qs: ApiQuerySet<serde_json::Value> = ApiQuerySet::new("/api/users/");
		assert_eq!(qs.build_url(), "/api/users/");
	}

	#[test]
	fn test_queryset_build_url_with_filters() {
		let qs: ApiQuerySet<serde_json::Value> = ApiQuerySet::new("/api/users/")
			.filter("is_active", true)
			.filter_op("age", FilterOp::Gte, 18);

		let url = qs.build_url();
		assert!(url.contains("is_active=true"));
		assert!(url.contains("age__gte=18"));
	}

	#[test]
	fn test_queryset_build_url_with_ordering() {
		let qs: ApiQuerySet<serde_json::Value> =
			ApiQuerySet::new("/api/users/").order_by(&["-created_at", "username"]);

		let url = qs.build_url();
		assert!(url.contains("ordering=-created_at%2Cusername"));
	}

	#[test]
	fn test_queryset_build_url_with_pagination() {
		let qs: ApiQuerySet<serde_json::Value> =
			ApiQuerySet::new("/api/users/").limit(10).offset(20);

		let url = qs.build_url();
		assert!(url.contains("limit=10"));
		assert!(url.contains("offset=20"));
	}

	#[test]
	fn test_queryset_build_url_with_fields() {
		let qs: ApiQuerySet<serde_json::Value> =
			ApiQuerySet::new("/api/users/").only(&["id", "username"]);

		let url = qs.build_url();
		assert!(url.contains("fields=id%2Cusername"));
	}

	#[test]
	fn test_queryset_chain() {
		let qs: ApiQuerySet<serde_json::Value> = ApiQuerySet::new("/api/users/")
			.filter("is_active", true)
			.exclude("role", "admin")
			.order_by(&["-created_at"])
			.limit(10)
			.offset(0);

		let url = qs.build_url();
		assert!(url.starts_with("/api/users/?"));
		assert!(url.contains("is_active=true"));
		assert!(url.contains("exclude__role=admin"));
		assert!(url.contains("ordering=-created_at"));
		assert!(url.contains("limit=10"));
	}

	#[test]
	fn test_filter_in_list() {
		let filter = Filter::with_op("id", FilterOp::In, vec![1, 2, 3]);
		let (key, value) = filter.to_query_param();
		assert_eq!(key, "id__in");
		assert_eq!(value, "1,2,3");
	}
}
