//! Django Admin Framework
//!
//! Auto-generated CRUD interface for models with:
//! - ModelAdmin configuration
//! - AdminSite registration
//! - List/Change/Add views
//! - Filters and search
//! - Bulk actions
//! - Permissions integration

use async_trait::async_trait;
use reinhardt_exception::{Error, Result};
use reinhardt_orm::Model;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Error type for admin operations
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
	#[error("Model not found: {0}")]
	ModelNotFound(String),

	#[error("Field not found: {0}")]
	FieldNotFound(String),

	#[error("Invalid filter: {0}")]
	InvalidFilter(String),

	#[error("Permission denied: {0}")]
	PermissionDenied(String),

	#[error("Query error: {0}")]
	QueryError(String),
}

impl From<AdminError> for Error {
	fn from(err: AdminError) -> Self {
		Error::Validation(err.to_string())
	}
}

/// Base trait for admin views
#[async_trait]
pub trait AdminView: Send + Sync {
	/// Render the admin view
	async fn render(&self) -> Result<String>;

	/// Check if the current user has permission to view this admin
	fn has_view_permission(&self) -> bool {
		true
	}

	/// Check if the current user has permission to add objects
	fn has_add_permission(&self) -> bool {
		true
	}

	/// Check if the current user has permission to change objects
	fn has_change_permission(&self) -> bool {
		true
	}

	/// Check if the current user has permission to delete objects
	fn has_delete_permission(&self) -> bool {
		true
	}
}

/// Django-style ModelAdmin for managing models
///
/// # Examples
///
/// ```
/// use reinhardt_views_core::admin::ModelAdmin;
/// use reinhardt_orm::Model;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct Article {
///     id: Option<i64>,
///     title: String,
///     content: String,
///     published: bool,
/// }
///
/// impl Model for Article {
///     type PrimaryKey = i64;
///     fn table_name() -> &'static str { "articles" }
///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// }
///
/// let admin = ModelAdmin::<Article>::new()
///     .with_list_display(vec!["id".to_string(), "title".to_string(), "published".to_string()])
///     .with_list_filter(vec!["published".to_string()])
///     .with_search_fields(vec!["title".to_string(), "content".to_string()]);
///
/// assert_eq!(admin.list_display(), &["id", "title", "published"]);
/// ```
pub struct ModelAdmin<T: Model> {
	/// Fields to display in the list view
	list_display: Vec<String>,
	/// Fields to filter by in the list view
	list_filter: Vec<String>,
	/// Fields to search in
	search_fields: Vec<String>,
	/// Ordering for the list view
	ordering: Vec<String>,
	/// Number of items per page
	list_per_page: usize,
	/// Whether to show full result count (can be slow for large tables)
	show_full_result_count: bool,
	/// Fields to display in readonly mode
	readonly_fields: Vec<String>,
	/// Custom queryset for filtering
	queryset: Option<Vec<T>>,
	/// PhantomData for type safety
	_phantom: PhantomData<T>,
}

impl<T: Model + Serialize + for<'de> Deserialize<'de> + Clone> ModelAdmin<T> {
	/// Creates a new ModelAdmin with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_views_core::admin::ModelAdmin;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     username: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let admin = ModelAdmin::<User>::new();
	/// assert_eq!(admin.list_per_page(), 100);
	/// ```
	pub fn new() -> Self {
		Self {
			list_display: vec![],
			list_filter: vec![],
			search_fields: vec![],
			ordering: vec![],
			list_per_page: 100,
			show_full_result_count: true,
			readonly_fields: vec![],
			queryset: None,
			_phantom: PhantomData,
		}
	}

	/// Sets the fields to display in the list view
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_views_core::admin::ModelAdmin;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct Article {
	///     id: Option<i64>,
	///     title: String,
	/// }
	///
	/// impl Model for Article {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "articles" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let admin = ModelAdmin::<Article>::new()
	///     .with_list_display(vec!["id".to_string(), "title".to_string()]);
	/// assert_eq!(admin.list_display().len(), 2);
	/// ```
	pub fn with_list_display(mut self, fields: Vec<String>) -> Self {
		self.list_display = fields;
		self
	}

	/// Sets the fields to filter by in the list view
	pub fn with_list_filter(mut self, fields: Vec<String>) -> Self {
		self.list_filter = fields;
		self
	}

	/// Sets the fields to search in
	pub fn with_search_fields(mut self, fields: Vec<String>) -> Self {
		self.search_fields = fields;
		self
	}

	/// Sets the ordering for the list view
	pub fn with_ordering(mut self, fields: Vec<String>) -> Self {
		self.ordering = fields;
		self
	}

	/// Sets the number of items per page
	pub fn with_list_per_page(mut self, count: usize) -> Self {
		self.list_per_page = count;
		self
	}

	/// Sets whether to show full result count
	pub fn with_show_full_result_count(mut self, show: bool) -> Self {
		self.show_full_result_count = show;
		self
	}

	/// Sets the readonly fields
	pub fn with_readonly_fields(mut self, fields: Vec<String>) -> Self {
		self.readonly_fields = fields;
		self
	}

	/// Sets a custom queryset for the admin
	pub fn with_queryset(mut self, queryset: Vec<T>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	/// Gets the list of fields to display
	pub fn list_display(&self) -> &[String] {
		&self.list_display
	}

	/// Gets the list of filter fields
	pub fn list_filter(&self) -> &[String] {
		&self.list_filter
	}

	/// Gets the search fields
	pub fn search_fields(&self) -> &[String] {
		&self.search_fields
	}

	/// Gets the ordering fields
	pub fn ordering(&self) -> &[String] {
		&self.ordering
	}

	/// Gets the number of items per page
	pub fn list_per_page(&self) -> usize {
		self.list_per_page
	}

	/// Gets whether to show full result count
	pub fn show_full_result_count(&self) -> bool {
		self.show_full_result_count
	}

	/// Gets the readonly fields
	pub fn readonly_fields(&self) -> &[String] {
		&self.readonly_fields
	}

	/// Gets the queryset for this admin
	pub async fn get_queryset(&self) -> Result<Vec<T>> {
		match &self.queryset {
			Some(qs) => Ok(qs.clone()),
			None => Ok(Vec::new()),
		}
	}

	/// Renders the list view as HTML
	pub async fn render_list(&self) -> Result<String> {
		let objects = self.get_queryset().await?;
		let count = objects.len();

		let mut html = String::from("<div class=\"admin-list\">\n");
		html.push_str(&format!("<h2>{} List</h2>\n", T::table_name()));
		html.push_str(&format!("<p>Total: {} items</p>\n", count));

		// Table header
		html.push_str("<table>\n<thead>\n<tr>\n");
		for field in &self.list_display {
			html.push_str(&format!("<th>{}</th>\n", field));
		}
		html.push_str("</tr>\n</thead>\n<tbody>\n");

		// Table rows
		for obj in objects {
			html.push_str("<tr>\n");
			let obj_json =
				serde_json::to_value(&obj).map_err(|e| Error::Serialization(e.to_string()))?;

			for field in &self.list_display {
				let value = obj_json
					.get(field)
					.map(|v| v.to_string())
					.unwrap_or_else(|| "-".to_string());
				html.push_str(&format!("<td>{}</td>\n", value));
			}
			html.push_str("</tr>\n");
		}

		html.push_str("</tbody>\n</table>\n</div>");

		Ok(html)
	}

	/// Searches the queryset based on search fields
	pub fn search(&self, query: &str, objects: Vec<T>) -> Vec<T> {
		if query.is_empty() || self.search_fields.is_empty() {
			return objects;
		}

		objects
			.into_iter()
			.filter(|obj| {
				let obj_json = serde_json::to_value(obj).ok();
				if let Some(json) = obj_json {
					self.search_fields.iter().any(|field| {
						json.get(field)
							.and_then(|v| v.as_str())
							.map(|s| s.to_lowercase().contains(&query.to_lowercase()))
							.unwrap_or(false)
					})
				} else {
					false
				}
			})
			.collect()
	}

	/// Filters the queryset based on filter criteria
	pub fn filter(&self, filters: &HashMap<String, String>, objects: Vec<T>) -> Vec<T> {
		if filters.is_empty() {
			return objects;
		}

		objects
			.into_iter()
			.filter(|obj| {
				let obj_json = serde_json::to_value(obj).ok();
				if let Some(json) = obj_json {
					filters.iter().all(|(field, value)| {
						json.get(field)
							.map(|v| {
								// Handle different value types
								match v {
									serde_json::Value::String(s) => s == value,
									serde_json::Value::Bool(b) => {
										value.to_lowercase() == b.to_string()
									}
									serde_json::Value::Number(n) => n.to_string() == *value,
									_ => v.to_string() == *value,
								}
							})
							.unwrap_or(false)
					})
				} else {
					false
				}
			})
			.collect()
	}
}

#[async_trait]
impl<T: Model + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync> AdminView
	for ModelAdmin<T>
{
	async fn render(&self) -> Result<String> {
		self.render_list().await
	}
}

impl<T: Model + Serialize + for<'de> Deserialize<'de> + Clone> Default for ModelAdmin<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestModel {
		id: Option<i64>,
		name: String,
		active: bool,
	}

	impl Model for TestModel {
		type PrimaryKey = i64;
		fn table_name() -> &'static str {
			"test_models"
		}
		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}
		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_model_admin_creation() {
		let admin = ModelAdmin::<TestModel>::new();
		assert_eq!(admin.list_per_page(), 100);
		assert!(admin.list_display().is_empty());
		assert!(admin.list_filter().is_empty());
		assert!(admin.search_fields().is_empty());
	}

	#[test]
	fn test_model_admin_with_list_display() {
		let admin = ModelAdmin::<TestModel>::new()
			.with_list_display(vec!["id".to_string(), "name".to_string()]);

		assert_eq!(admin.list_display().len(), 2);
		assert_eq!(admin.list_display()[0], "id");
		assert_eq!(admin.list_display()[1], "name");
	}

	#[test]
	fn test_model_admin_with_filters() {
		let admin = ModelAdmin::<TestModel>::new()
			.with_list_filter(vec!["active".to_string()])
			.with_search_fields(vec!["name".to_string()]);

		assert_eq!(admin.list_filter().len(), 1);
		assert_eq!(admin.search_fields().len(), 1);
	}

	#[test]
	fn test_model_admin_search() {
		let admin = ModelAdmin::<TestModel>::new().with_search_fields(vec!["name".to_string()]);

		let objects = vec![
			TestModel {
				id: Some(1),
				name: "Alice".to_string(),
				active: true,
			},
			TestModel {
				id: Some(2),
				name: "Bob".to_string(),
				active: false,
			},
			TestModel {
				id: Some(3),
				name: "Charlie".to_string(),
				active: true,
			},
		];

		let results = admin.search("ali", objects);
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].name, "Alice");
	}

	#[test]
	fn test_model_admin_filter() {
		let admin = ModelAdmin::<TestModel>::new().with_list_filter(vec!["active".to_string()]);

		let objects = vec![
			TestModel {
				id: Some(1),
				name: "Alice".to_string(),
				active: true,
			},
			TestModel {
				id: Some(2),
				name: "Bob".to_string(),
				active: false,
			},
			TestModel {
				id: Some(3),
				name: "Charlie".to_string(),
				active: true,
			},
		];

		let mut filters = HashMap::new();
		filters.insert("active".to_string(), "true".to_string());

		let results = admin.filter(&filters, objects);
		assert_eq!(results.len(), 2);
		assert!(results.iter().all(|r| r.active));
	}

	#[tokio::test]
	async fn test_model_admin_get_queryset() {
		let objects = vec![TestModel {
			id: Some(1),
			name: "Test".to_string(),
			active: true,
		}];

		let admin = ModelAdmin::<TestModel>::new().with_queryset(objects.clone());

		let queryset = admin.get_queryset().await.unwrap();
		assert_eq!(queryset.len(), 1);
		assert_eq!(queryset[0].name, "Test");
	}

	#[tokio::test]
	async fn test_model_admin_render_list() {
		let objects = vec![
			TestModel {
				id: Some(1),
				name: "Alice".to_string(),
				active: true,
			},
			TestModel {
				id: Some(2),
				name: "Bob".to_string(),
				active: false,
			},
		];

		let admin = ModelAdmin::<TestModel>::new()
			.with_queryset(objects)
			.with_list_display(vec!["id".to_string(), "name".to_string()]);

		let html = admin.render_list().await.unwrap();

		assert!(html.contains("<div class=\"admin-list\">"));
		assert!(html.contains("test_models List"));
		assert!(html.contains("Total: 2 items"));
		assert!(html.contains("<th>id</th>"));
		assert!(html.contains("<th>name</th>"));
	}

	#[tokio::test]
	async fn test_model_admin_render_via_trait() {
		let objects = vec![TestModel {
			id: Some(1),
			name: "Test".to_string(),
			active: true,
		}];

		let admin = ModelAdmin::<TestModel>::new().with_queryset(objects);

		let html = admin.render().await.unwrap();
		assert!(html.contains("<div class=\"admin-list\">"));
	}

	#[test]
	fn test_model_admin_permissions() {
		let admin = ModelAdmin::<TestModel>::new();

		assert!(admin.has_view_permission());
		assert!(admin.has_add_permission());
		assert!(admin.has_change_permission());
		assert!(admin.has_delete_permission());
	}
}
