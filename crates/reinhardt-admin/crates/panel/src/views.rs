//! Admin views for CRUD operations
//!
//! This module provides view classes for displaying and manipulating model instances
//! in the admin interface.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context data for admin views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminViewContext {
	/// The model name
	pub model_name: String,
	/// The view title
	pub title: String,
	/// Additional context data
	pub extra: HashMap<String, serde_json::Value>,
}

impl AdminViewContext {
	/// Create a new view context
	pub fn new(model_name: impl Into<String>, title: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			title: title.into(),
			extra: HashMap::new(),
		}
	}

	/// Add extra context data
	pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.extra.insert(key.into(), value);
		self
	}

	/// Get extra context value
	pub fn get_extra(&self, key: &str) -> Option<&serde_json::Value> {
		self.extra.get(key)
	}
}

/// List view for displaying multiple model instances
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::ListView;
///
/// let view = ListView::new("User");
/// assert_eq!(view.model_name(), "User");
/// assert_eq!(view.get_page_size(), 100);
/// ```
#[derive(Debug, Clone)]
pub struct ListView {
	model_name: String,
	page_size: usize,
	ordering: Vec<String>,
	search_fields: Vec<String>,
}

impl ListView {
	/// Create a new list view
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			page_size: 100,
			ordering: vec!["-id".to_string()],
			search_fields: Vec::new(),
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Set the page size
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_panel::ListView;
	///
	/// let view = ListView::new("User").with_page_size(50);
	/// assert_eq!(view.get_page_size(), 50);
	/// ```
	pub fn with_page_size(mut self, size: usize) -> Self {
		self.page_size = size;
		self
	}

	/// Get the page size
	pub fn get_page_size(&self) -> usize {
		self.page_size
	}

	/// Set ordering fields
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_panel::ListView;
	///
	/// let view = ListView::new("User")
	///     .with_ordering(vec!["name".to_string(), "-created_at".to_string()]);
	/// ```
	pub fn with_ordering(mut self, ordering: Vec<String>) -> Self {
		self.ordering = ordering;
		self
	}

	/// Get ordering fields
	pub fn get_ordering(&self) -> &[String] {
		&self.ordering
	}

	/// Set search fields
	pub fn with_search_fields(mut self, fields: Vec<String>) -> Self {
		self.search_fields = fields;
		self
	}

	/// Get search fields
	pub fn get_search_fields(&self) -> &[String] {
		&self.search_fields
	}

	/// Build the view context
	pub fn build_context(&self) -> AdminViewContext {
		AdminViewContext::new(&self.model_name, format!("{} List", self.model_name))
	}
}

/// Detail view for displaying a single model instance
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::DetailView;
///
/// let view = DetailView::new("User", "123");
/// assert_eq!(view.model_name(), "User");
/// assert_eq!(view.object_id(), "123");
/// ```
#[derive(Debug, Clone)]
pub struct DetailView {
	model_name: String,
	object_id: String,
	fields: Option<Vec<String>>,
}

impl DetailView {
	/// Create a new detail view
	pub fn new(model_name: impl Into<String>, object_id: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			object_id: object_id.into(),
			fields: None,
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Get the object ID
	pub fn object_id(&self) -> &str {
		&self.object_id
	}

	/// Set fields to display
	pub fn with_fields(mut self, fields: Vec<String>) -> Self {
		self.fields = Some(fields);
		self
	}

	/// Get fields to display
	pub fn get_fields(&self) -> Option<&[String]> {
		self.fields.as_deref()
	}

	/// Build the view context
	pub fn build_context(&self) -> AdminViewContext {
		AdminViewContext::new(&self.model_name, format!("{} Detail", self.model_name))
	}
}

/// Create view for adding new model instances
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::CreateView;
///
/// let view = CreateView::new("User");
/// assert_eq!(view.model_name(), "User");
/// ```
#[derive(Debug, Clone)]
pub struct CreateView {
	model_name: String,
	fields: Option<Vec<String>>,
	initial_data: HashMap<String, serde_json::Value>,
}

impl CreateView {
	/// Create a new create view
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			fields: None,
			initial_data: HashMap::new(),
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Set fields to display in the form
	pub fn with_fields(mut self, fields: Vec<String>) -> Self {
		self.fields = Some(fields);
		self
	}

	/// Get fields to display
	pub fn get_fields(&self) -> Option<&[String]> {
		self.fields.as_deref()
	}

	/// Set initial data for the form
	pub fn with_initial(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.initial_data.insert(key.into(), value);
		self
	}

	/// Get initial data
	pub fn get_initial_data(&self) -> &HashMap<String, serde_json::Value> {
		&self.initial_data
	}

	/// Build the view context
	pub fn build_context(&self) -> AdminViewContext {
		AdminViewContext::new(&self.model_name, format!("Add {}", self.model_name))
	}
}

/// Update view for editing existing model instances
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::UpdateView;
///
/// let view = UpdateView::new("User", "123");
/// assert_eq!(view.model_name(), "User");
/// assert_eq!(view.object_id(), "123");
/// ```
#[derive(Debug, Clone)]
pub struct UpdateView {
	model_name: String,
	object_id: String,
	fields: Option<Vec<String>>,
	readonly_fields: Vec<String>,
}

impl UpdateView {
	/// Create a new update view
	pub fn new(model_name: impl Into<String>, object_id: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			object_id: object_id.into(),
			fields: None,
			readonly_fields: Vec::new(),
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Get the object ID
	pub fn object_id(&self) -> &str {
		&self.object_id
	}

	/// Set fields to display in the form
	pub fn with_fields(mut self, fields: Vec<String>) -> Self {
		self.fields = Some(fields);
		self
	}

	/// Get fields to display
	pub fn get_fields(&self) -> Option<&[String]> {
		self.fields.as_deref()
	}

	/// Set readonly fields
	pub fn with_readonly_fields(mut self, fields: Vec<String>) -> Self {
		self.readonly_fields = fields;
		self
	}

	/// Get readonly fields
	pub fn get_readonly_fields(&self) -> &[String] {
		&self.readonly_fields
	}

	/// Build the view context
	pub fn build_context(&self) -> AdminViewContext {
		AdminViewContext::new(&self.model_name, format!("Change {}", self.model_name))
	}
}

/// Delete view for removing model instances
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::DeleteView;
///
/// let view = DeleteView::new("User", "123");
/// assert_eq!(view.model_name(), "User");
/// assert_eq!(view.object_id(), "123");
/// assert!(view.requires_confirmation());
/// ```
#[derive(Debug, Clone)]
pub struct DeleteView {
	model_name: String,
	object_id: String,
	cascade_info: Option<CascadeInfo>,
}

impl DeleteView {
	/// Create a new delete view
	pub fn new(model_name: impl Into<String>, object_id: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			object_id: object_id.into(),
			cascade_info: None,
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Get the object ID
	pub fn object_id(&self) -> &str {
		&self.object_id
	}

	/// Check if deletion requires confirmation
	pub fn requires_confirmation(&self) -> bool {
		true // Always require confirmation for safety
	}

	/// Set cascade deletion information
	pub fn with_cascade_info(mut self, info: CascadeInfo) -> Self {
		self.cascade_info = Some(info);
		self
	}

	/// Get cascade deletion information
	pub fn get_cascade_info(&self) -> Option<&CascadeInfo> {
		self.cascade_info.as_ref()
	}

	/// Build the view context
	pub fn build_context(&self) -> AdminViewContext {
		let mut context =
			AdminViewContext::new(&self.model_name, format!("Delete {}", self.model_name));

		if let Some(cascade) = &self.cascade_info {
			context = context.with_extra(
				"cascade_info",
				serde_json::json!({
					"related_objects": cascade.related_objects,
					"total_count": cascade.total_count,
				}),
			);
		}

		context
	}
}

/// Information about cascade deletions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeInfo {
	/// Related objects that will be deleted
	pub related_objects: Vec<RelatedObject>,
	/// Total number of objects to be deleted
	pub total_count: usize,
}

impl CascadeInfo {
	/// Create new cascade info
	pub fn new() -> Self {
		Self {
			related_objects: Vec::new(),
			total_count: 0,
		}
	}

	/// Add a related object
	pub fn add_related(mut self, model: impl Into<String>, count: usize) -> Self {
		self.related_objects.push(RelatedObject {
			model: model.into(),
			count,
		});
		self.total_count += count;
		self
	}
}

impl Default for CascadeInfo {
	fn default() -> Self {
		Self::new()
	}
}

/// Information about a related object that will be cascade deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedObject {
	/// Model name
	pub model: String,
	/// Number of objects to be deleted
	pub count: usize,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_admin_view_context() {
		let context = AdminViewContext::new("User", "User List");
		assert_eq!(context.model_name, "User");
		assert_eq!(context.title, "User List");
		assert!(context.extra.is_empty());
	}

	#[test]
	fn test_admin_view_context_with_extra() {
		let context = AdminViewContext::new("User", "User List")
			.with_extra("page", serde_json::json!(1))
			.with_extra("page_size", serde_json::json!(50));

		assert_eq!(context.extra.len(), 2);
		assert_eq!(context.get_extra("page"), Some(&serde_json::json!(1)));
	}

	#[test]
	fn test_list_view_new() {
		let view = ListView::new("User");
		assert_eq!(view.model_name(), "User");
		assert_eq!(view.get_page_size(), 100);
		assert_eq!(view.get_ordering(), &["-id".to_string()]);
	}

	#[test]
	fn test_list_view_with_page_size() {
		let view = ListView::new("User").with_page_size(50);
		assert_eq!(view.get_page_size(), 50);
	}

	#[test]
	fn test_list_view_with_ordering() {
		let view = ListView::new("User")
			.with_ordering(vec!["name".to_string(), "-created_at".to_string()]);
		assert_eq!(
			view.get_ordering(),
			&["name".to_string(), "-created_at".to_string()]
		);
	}

	#[test]
	fn test_list_view_build_context() {
		let view = ListView::new("User");
		let context = view.build_context();
		assert_eq!(context.model_name, "User");
		assert_eq!(context.title, "User List");
	}

	#[test]
	fn test_detail_view_new() {
		let view = DetailView::new("User", "123");
		assert_eq!(view.model_name(), "User");
		assert_eq!(view.object_id(), "123");
		assert!(view.get_fields().is_none());
	}

	#[test]
	fn test_detail_view_with_fields() {
		let view =
			DetailView::new("User", "123").with_fields(vec!["id".to_string(), "name".to_string()]);
		assert_eq!(
			view.get_fields(),
			Some(&["id".to_string(), "name".to_string()][..])
		);
	}

	#[test]
	fn test_create_view_new() {
		let view = CreateView::new("User");
		assert_eq!(view.model_name(), "User");
		assert!(view.get_initial_data().is_empty());
	}

	#[test]
	fn test_create_view_with_initial() {
		let view = CreateView::new("User")
			.with_initial("is_active", serde_json::json!(true))
			.with_initial("role", serde_json::json!("user"));

		assert_eq!(view.get_initial_data().len(), 2);
		assert_eq!(
			view.get_initial_data().get("is_active"),
			Some(&serde_json::json!(true))
		);
	}

	#[test]
	fn test_update_view_new() {
		let view = UpdateView::new("User", "123");
		assert_eq!(view.model_name(), "User");
		assert_eq!(view.object_id(), "123");
		assert!(view.get_readonly_fields().is_empty());
	}

	#[test]
	fn test_update_view_with_readonly_fields() {
		let view = UpdateView::new("User", "123")
			.with_readonly_fields(vec!["id".to_string(), "created_at".to_string()]);
		assert_eq!(
			view.get_readonly_fields(),
			&["id".to_string(), "created_at".to_string()]
		);
	}

	#[test]
	fn test_delete_view_new() {
		let view = DeleteView::new("User", "123");
		assert_eq!(view.model_name(), "User");
		assert_eq!(view.object_id(), "123");
		assert!(view.requires_confirmation());
		assert!(view.get_cascade_info().is_none());
	}

	#[test]
	fn test_cascade_info() {
		let info = CascadeInfo::new()
			.add_related("Post", 5)
			.add_related("Comment", 12);

		assert_eq!(info.total_count, 17);
		assert_eq!(info.related_objects.len(), 2);
		assert_eq!(info.related_objects[0].model, "Post");
		assert_eq!(info.related_objects[0].count, 5);
	}

	#[test]
	fn test_delete_view_with_cascade_info() {
		let cascade = CascadeInfo::new().add_related("Post", 3);
		let view = DeleteView::new("User", "123").with_cascade_info(cascade);

		let info = view.get_cascade_info().unwrap();
		assert_eq!(info.total_count, 3);
	}

	#[test]
	fn test_delete_view_build_context_with_cascade() {
		let cascade = CascadeInfo::new()
			.add_related("Post", 3)
			.add_related("Comment", 7);
		let view = DeleteView::new("User", "123").with_cascade_info(cascade);

		let context = view.build_context();
		assert!(context.get_extra("cascade_info").is_some());
	}
}
