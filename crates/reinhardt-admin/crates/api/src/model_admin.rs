//! Model admin configuration and trait
//!
//! This module defines how models are displayed and managed in the admin interface.

use async_trait::async_trait;

#[cfg(feature = "contenttypes")]
use reinhardt_db::contenttypes::{ContentType, PermissionAction};

/// Trait for configuring model administration
///
/// Implement this trait to customize how a model is displayed and edited in the admin.
#[async_trait]
pub trait ModelAdmin: Send + Sync {
	/// Get the model name
	fn model_name(&self) -> &str;

	/// Get the database table name
	///
	/// By default, returns the model name in lowercase.
	fn table_name(&self) -> &str {
		// Default implementation returns empty string
		// Override in implementations to return actual table name
		""
	}

	/// Get the primary key field name
	///
	/// By default, returns "id".
	fn pk_field(&self) -> &str {
		"id"
	}

	/// Fields to display in list view
	fn list_display(&self) -> Vec<&str> {
		vec!["id"]
	}

	/// Fields that can be used for filtering
	fn list_filter(&self) -> Vec<&str> {
		vec![]
	}

	/// Fields that can be searched
	fn search_fields(&self) -> Vec<&str> {
		vec![]
	}

	/// Fields to display in forms (None = all fields)
	fn fields(&self) -> Option<Vec<&str>> {
		None
	}

	/// Read-only fields
	fn readonly_fields(&self) -> Vec<&str> {
		vec![]
	}

	/// Ordering for list view (prefix with "-" for descending)
	fn ordering(&self) -> Vec<&str> {
		vec!["-id"]
	}

	/// Number of items per page (None = use site default)
	fn list_per_page(&self) -> Option<usize> {
		None
	}

	/// Check if user has permission to view this model
	///
	/// Default implementation allows all access.
	/// Override this method to implement custom permission checking.
	async fn has_view_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
		true
	}

	/// Check if user has permission to add instances
	///
	/// Default implementation allows all access.
	/// Override this method to implement custom permission checking.
	async fn has_add_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
		true
	}

	/// Check if user has permission to change instances
	///
	/// Default implementation allows all access.
	/// Override this method to implement custom permission checking.
	async fn has_change_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
		true
	}

	/// Check if user has permission to delete instances
	///
	/// Default implementation allows all access.
	/// Override this method to implement custom permission checking.
	async fn has_delete_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
		true
	}

	/// Get the application label for this model
	///
	/// Used for permission checking and ContentType registration.
	/// By default, returns "admin".
	fn app_label(&self) -> &str {
		"admin"
	}

	/// Get the ContentType for this model
	///
	/// Returns a ContentType based on the app_label and model_name.
	/// This is used for permission checking and generic relations.
	#[cfg(feature = "contenttypes")]
	fn get_content_type(&self) -> ContentType {
		ContentType::new(self.app_label(), self.model_name())
	}

	/// Check if user has a specific permission for this model's ContentType
	///
	/// Uses the ContentType-based permission system for fine-grained access control.
	/// Default implementation always returns true.
	#[cfg(feature = "contenttypes")]
	async fn has_content_type_permission(
		&self,
		_action: PermissionAction,
		_user: &(dyn std::any::Any + Send + Sync),
	) -> bool {
		true
	}

	/// Get the permission string for a specific action
	///
	/// Returns a Django-style permission string like "app_label.action_model".
	#[cfg(feature = "contenttypes")]
	fn get_permission_codename(&self, action: PermissionAction) -> String {
		use reinhardt_db::contenttypes::ContentTypePermission;
		ContentTypePermission::format(&self.get_content_type(), action)
	}
}

/// Configuration-based model admin implementation
///
/// Provides a simple way to configure model admin without implementing the trait.
///
/// # Examples
///
/// ```
/// use reinhardt_admin_api::{ModelAdminConfig, ModelAdmin};
///
/// let admin = ModelAdminConfig::builder()
///     .model_name("User")
///     .list_display(vec!["id", "username", "email"])
///     .list_filter(vec!["is_active"])
///     .search_fields(vec!["username", "email"])
///     .build();
///
/// assert_eq!(admin.model_name(), "User");
/// ```
#[derive(Debug, Clone)]
pub struct ModelAdminConfig {
	model_name: String,
	app_label: String,
	table_name: Option<String>,
	pk_field: String,
	list_display: Vec<String>,
	list_filter: Vec<String>,
	search_fields: Vec<String>,
	fields: Option<Vec<String>>,
	readonly_fields: Vec<String>,
	ordering: Vec<String>,
	list_per_page: Option<usize>,
}

impl ModelAdminConfig {
	/// Create a new model admin configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_api::{ModelAdminConfig, ModelAdmin};
	///
	/// let admin = ModelAdminConfig::new("User");
	/// assert_eq!(admin.model_name(), "User");
	/// assert_eq!(admin.app_label(), "admin");
	/// ```
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			app_label: "admin".into(),
			table_name: None,
			pk_field: "id".into(),
			list_display: vec!["id".into()],
			list_filter: vec![],
			search_fields: vec![],
			fields: None,
			readonly_fields: vec![],
			ordering: vec!["-id".into()],
			list_per_page: None,
		}
	}

	/// Set the application label
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_api::{ModelAdminConfig, ModelAdmin};
	///
	/// let admin = ModelAdminConfig::new("User").with_app_label("accounts");
	/// assert_eq!(admin.app_label(), "accounts");
	/// ```
	pub fn with_app_label(mut self, label: impl Into<String>) -> Self {
		self.app_label = label.into();
		self
	}

	/// Start building a model admin configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_api::ModelAdminConfig;
	///
	/// let admin = ModelAdminConfig::builder()
	///     .model_name("User")
	///     .list_display(vec!["id", "username"])
	///     .build();
	/// ```
	pub fn builder() -> ModelAdminConfigBuilder {
		ModelAdminConfigBuilder::default()
	}

	/// Set list display fields
	pub fn with_list_display(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.list_display = fields.into_iter().map(Into::into).collect();
		self
	}

	/// Set list filter fields
	pub fn with_list_filter(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.list_filter = fields.into_iter().map(Into::into).collect();
		self
	}

	/// Set search fields
	pub fn with_search_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.search_fields = fields.into_iter().map(Into::into).collect();
		self
	}
}

#[async_trait]
impl ModelAdmin for ModelAdminConfig {
	fn model_name(&self) -> &str {
		&self.model_name
	}

	fn app_label(&self) -> &str {
		&self.app_label
	}

	fn table_name(&self) -> &str {
		self.table_name
			.as_deref()
			.unwrap_or(self.model_name.as_str())
	}

	fn pk_field(&self) -> &str {
		&self.pk_field
	}

	fn list_display(&self) -> Vec<&str> {
		self.list_display.iter().map(|s| s.as_str()).collect()
	}

	fn list_filter(&self) -> Vec<&str> {
		self.list_filter.iter().map(|s| s.as_str()).collect()
	}

	fn search_fields(&self) -> Vec<&str> {
		self.search_fields.iter().map(|s| s.as_str()).collect()
	}

	fn fields(&self) -> Option<Vec<&str>> {
		self.fields
			.as_ref()
			.map(|f| f.iter().map(|s| s.as_str()).collect())
	}

	fn readonly_fields(&self) -> Vec<&str> {
		self.readonly_fields.iter().map(|s| s.as_str()).collect()
	}

	fn ordering(&self) -> Vec<&str> {
		self.ordering.iter().map(|s| s.as_str()).collect()
	}

	fn list_per_page(&self) -> Option<usize> {
		self.list_per_page
	}
}

/// Builder for ModelAdminConfig
#[derive(Debug, Default)]
pub struct ModelAdminConfigBuilder {
	model_name: Option<String>,
	app_label: Option<String>,
	table_name: Option<String>,
	pk_field: Option<String>,
	list_display: Option<Vec<String>>,
	list_filter: Option<Vec<String>>,
	search_fields: Option<Vec<String>>,
	fields: Option<Vec<String>>,
	readonly_fields: Option<Vec<String>>,
	ordering: Option<Vec<String>>,
	list_per_page: Option<usize>,
}

impl ModelAdminConfigBuilder {
	/// Set the model name
	pub fn model_name(mut self, name: impl Into<String>) -> Self {
		self.model_name = Some(name.into());
		self
	}

	/// Set the application label
	///
	/// If not set, defaults to "admin".
	pub fn app_label(mut self, label: impl Into<String>) -> Self {
		self.app_label = Some(label.into());
		self
	}

	/// Set the database table name
	///
	/// If not set, defaults to the model name.
	pub fn table_name(mut self, name: impl Into<String>) -> Self {
		self.table_name = Some(name.into());
		self
	}

	/// Set the primary key field name
	///
	/// If not set, defaults to "id".
	pub fn pk_field(mut self, field: impl Into<String>) -> Self {
		self.pk_field = Some(field.into());
		self
	}

	/// Set list display fields
	pub fn list_display(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.list_display = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set list filter fields
	pub fn list_filter(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.list_filter = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set search fields
	pub fn search_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.search_fields = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set form fields
	pub fn fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.fields = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set readonly fields
	pub fn readonly_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.readonly_fields = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set ordering
	pub fn ordering(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.ordering = Some(fields.into_iter().map(Into::into).collect());
		self
	}

	/// Set items per page
	pub fn list_per_page(mut self, count: usize) -> Self {
		self.list_per_page = Some(count);
		self
	}

	/// Build the configuration
	///
	/// # Panics
	///
	/// Panics if model_name is not set
	pub fn build(self) -> ModelAdminConfig {
		ModelAdminConfig {
			model_name: self.model_name.expect("model_name is required"),
			app_label: self.app_label.unwrap_or_else(|| "admin".into()),
			table_name: self.table_name,
			pk_field: self.pk_field.unwrap_or_else(|| "id".into()),
			list_display: self.list_display.unwrap_or_else(|| vec!["id".into()]),
			list_filter: self.list_filter.unwrap_or_default(),
			search_fields: self.search_fields.unwrap_or_default(),
			fields: self.fields,
			readonly_fields: self.readonly_fields.unwrap_or_default(),
			ordering: self.ordering.unwrap_or_else(|| vec!["-id".into()]),
			list_per_page: self.list_per_page,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_model_admin_config_creation() {
		let admin = ModelAdminConfig::new("User");
		assert_eq!(admin.model_name(), "User");
		assert_eq!(admin.list_display(), vec!["id"]);
		assert_eq!(admin.list_filter(), Vec::<&str>::new());
	}

	#[test]
	fn test_model_admin_config_builder() {
		let admin = ModelAdminConfig::builder()
			.model_name("User")
			.list_display(vec!["id", "username", "email"])
			.list_filter(vec!["is_active"])
			.search_fields(vec!["username", "email"])
			.list_per_page(50)
			.build();

		assert_eq!(admin.model_name(), "User");
		assert_eq!(admin.list_display(), vec!["id", "username", "email"]);
		assert_eq!(admin.list_filter(), vec!["is_active"]);
		assert_eq!(admin.search_fields(), vec!["username", "email"]);
		assert_eq!(admin.list_per_page(), Some(50));
	}

	#[test]
	fn test_with_methods() {
		let admin = ModelAdminConfig::new("Post")
			.with_list_display(vec!["id", "title", "author"])
			.with_list_filter(vec!["status", "created_at"])
			.with_search_fields(vec!["title", "content"]);

		assert_eq!(admin.list_display(), vec!["id", "title", "author"]);
		assert_eq!(admin.list_filter(), vec!["status", "created_at"]);
		assert_eq!(admin.search_fields(), vec!["title", "content"]);
	}

	#[test]
	#[should_panic(expected = "model_name is required")]
	fn test_builder_without_model_name() {
		ModelAdminConfig::builder().build();
	}
}
