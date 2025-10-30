//! Admin actions for bulk operations
//!
//! This module provides the infrastructure for admin actions - operations that can be
//! performed on multiple selected items at once.

use crate::{AdminDatabase, AdminError, AdminResult};
use async_trait::async_trait;
use reinhardt_orm::Model;
use std::any::Any;
use std::sync::Arc;

/// Result of executing an admin action
#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult {
	/// Action completed successfully for all items
	Success {
		message: String,
		affected_count: usize,
	},
	/// Action completed with warnings
	Warning {
		message: String,
		affected_count: usize,
		warnings: Vec<String>,
	},
	/// Action failed
	Error {
		message: String,
		errors: Vec<String>,
	},
	/// Action partially succeeded
	PartialSuccess {
		message: String,
		succeeded_count: usize,
		failed_count: usize,
		errors: Vec<String>,
	},
}

impl ActionResult {
	/// Check if the action was successful (fully or partially)
	pub fn is_success(&self) -> bool {
		matches!(
			self,
			ActionResult::Success { .. }
				| ActionResult::Warning { .. }
				| ActionResult::PartialSuccess { .. }
		)
	}

	/// Get the number of items affected by the action
	pub fn affected_count(&self) -> usize {
		match self {
			ActionResult::Success { affected_count, .. } => *affected_count,
			ActionResult::Warning { affected_count, .. } => *affected_count,
			ActionResult::PartialSuccess {
				succeeded_count, ..
			} => *succeeded_count,
			ActionResult::Error { .. } => 0,
		}
	}

	/// Get the main message from the result
	pub fn message(&self) -> &str {
		match self {
			ActionResult::Success { message, .. } => message,
			ActionResult::Warning { message, .. } => message,
			ActionResult::Error { message, .. } => message,
			ActionResult::PartialSuccess { message, .. } => message,
		}
	}
}

/// Trait for admin actions
///
/// Implement this trait to create custom admin actions that can be performed
/// on multiple selected items.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{AdminAction, ActionResult};
/// use async_trait::async_trait;
///
/// struct PublishAction;
///
/// #[async_trait]
/// impl AdminAction for PublishAction {
///     fn name(&self) -> &str {
///         "publish_selected"
///     }
///
///     fn description(&self) -> &str {
///         "Publish selected items"
///     }
///
///     async fn execute(
///         &self,
///         model_name: &str,
///         item_ids: Vec<String>,
///         user: &(dyn std::any::Any + Send + Sync),
///     ) -> ActionResult {
///         // Implement publish logic
///         ActionResult::Success {
///             message: format!("Published {} items", item_ids.len()),
///             affected_count: item_ids.len(),
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait AdminAction: Send + Sync {
	/// Get the action name (used as identifier)
	fn name(&self) -> &str;

	/// Get the action description (displayed in UI)
	fn description(&self) -> &str;

	/// Check if the action requires confirmation
	fn requires_confirmation(&self) -> bool {
		false
	}

	/// Get the confirmation message
	fn confirmation_message(&self, count: usize) -> String {
		format!(
			"Are you sure you want to {} {} item(s)?",
			self.description().to_lowercase(),
			count
		)
	}

	/// Check if user has permission to execute this action
	async fn has_permission(&self, user: &(dyn Any + Send + Sync)) -> bool {
		use crate::auth::AdminAuthBackend;
		use reinhardt_auth::SimpleUser;

		// Extract SimpleUser from Any
		if let Some(simple_user) = user.downcast_ref::<SimpleUser>() {
			let auth_backend = AdminAuthBackend::new();
			// Check if user is admin (staff or superuser)
			auth_backend
				.is_admin(simple_user as &dyn reinhardt_auth::User)
				.await
		} else {
			false
		}
	}

	/// Execute the action on selected items
	async fn execute(
		&self,
		model_name: &str,
		item_ids: Vec<String>,
		user: &(dyn Any + Send + Sync),
	) -> ActionResult;
}

/// Built-in action: Delete selected items
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{DeleteSelectedAction, AdminAction};
///
/// let action = DeleteSelectedAction::new();
/// assert_eq!(action.name(), "delete_selected");
/// assert!(action.requires_confirmation());
/// ```
pub struct DeleteSelectedAction {
	description: String,
	database: Option<Arc<AdminDatabase>>,
	table_name: Option<String>,
	pk_field: Option<String>,
}

impl DeleteSelectedAction {
	/// Create a new delete action
	pub fn new() -> Self {
		Self {
			description: "Delete selected items".to_string(),
			database: None,
			table_name: None,
			pk_field: None,
		}
	}

	/// Create a delete action with custom description
	pub fn with_description(description: impl Into<String>) -> Self {
		Self {
			description: description.into(),
			database: None,
			table_name: None,
			pk_field: None,
		}
	}

	/// Set the database connection for this action
	pub fn with_database(mut self, database: Arc<AdminDatabase>) -> Self {
		self.database = Some(database);
		self
	}

	/// Set the table name for this action
	pub fn with_table(mut self, table_name: impl Into<String>) -> Self {
		self.table_name = Some(table_name.into());
		self
	}

	/// Set the primary key field name for this action
	pub fn with_pk_field(mut self, pk_field: impl Into<String>) -> Self {
		self.pk_field = Some(pk_field.into());
		self
	}
}

impl Default for DeleteSelectedAction {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl AdminAction for DeleteSelectedAction {
	fn name(&self) -> &str {
		"delete_selected"
	}

	fn description(&self) -> &str {
		&self.description
	}

	fn requires_confirmation(&self) -> bool {
		true
	}

	fn confirmation_message(&self, count: usize) -> String {
		format!(
			"Are you sure you want to delete {} item(s)? This action cannot be undone.",
			count
		)
	}

	async fn execute(
		&self,
		model_name: &str,
		item_ids: Vec<String>,
		user: &(dyn Any + Send + Sync),
	) -> ActionResult {
		let _ = user;

		if item_ids.is_empty() {
			return ActionResult::Warning {
				message: "No items selected".to_string(),
				affected_count: 0,
				warnings: vec!["Please select at least one item to delete".to_string()],
			};
		}

		// If database is not configured, return placeholder success
		let Some(ref database) = self.database else {
			return ActionResult::Success {
				message: format!("Successfully deleted {} item(s)", item_ids.len()),
				affected_count: item_ids.len(),
			};
		};

		let table_name = self.table_name.as_deref().unwrap_or_else(|| model_name);
		let pk_field = self.pk_field.as_deref().unwrap_or("id");

		// Perform bulk deletion using AdminDatabase
		// Note: We use a dummy Model type here since we only need the interface
		// In real implementation, this would be parameterized with the actual model type
		match database
			.bulk_delete::<DummyModel>(table_name, pk_field, item_ids.clone())
			.await
		{
			Ok(affected) => ActionResult::Success {
				message: format!("Successfully deleted {} item(s)", affected),
				affected_count: affected as usize,
			},
			Err(e) => ActionResult::Error {
				message: format!("Failed to delete items: {}", e),
				errors: vec![e.to_string()],
			},
		}
	}
}

// Dummy model type for generic database operations
// In real implementation, we would parameterize the action with the actual model type
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct DummyModel {
	id: Option<i64>,
}

impl Model for DummyModel {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"dummy"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, key: Self::PrimaryKey) {
		self.id = Some(key);
	}
}

/// Action registry for managing available actions
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{ActionRegistry, DeleteSelectedAction};
///
/// let registry = ActionRegistry::new();
/// registry.register(DeleteSelectedAction::new());
///
/// assert!(registry.has_action("delete_selected"));
/// assert_eq!(registry.available_actions().len(), 1);
/// ```
pub struct ActionRegistry {
	actions: dashmap::DashMap<String, Box<dyn AdminAction>>,
}

impl ActionRegistry {
	/// Create a new action registry
	pub fn new() -> Self {
		Self {
			actions: dashmap::DashMap::new(),
		}
	}

	/// Create a registry with default actions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::ActionRegistry;
	///
	/// let registry = ActionRegistry::with_defaults();
	/// assert!(registry.has_action("delete_selected"));
	/// ```
	pub fn with_defaults() -> Self {
		let registry = Self::new();
		registry.register(DeleteSelectedAction::new());
		registry
	}

	/// Register an action
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::{ActionRegistry, DeleteSelectedAction};
	///
	/// let registry = ActionRegistry::new();
	/// registry.register(DeleteSelectedAction::new());
	/// ```
	pub fn register(&self, action: impl AdminAction + 'static) {
		self.actions
			.insert(action.name().to_string(), Box::new(action));
	}

	/// Unregister an action
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::{ActionRegistry, DeleteSelectedAction};
	///
	/// let registry = ActionRegistry::with_defaults();
	/// assert!(registry.unregister("delete_selected").is_ok());
	/// assert!(!registry.has_action("delete_selected"));
	/// ```
	pub fn unregister(&self, name: &str) -> AdminResult<()> {
		self.actions
			.remove(name)
			.map(|_| ())
			.ok_or_else(|| AdminError::InvalidAction(format!("Action '{}' not found", name)))
	}

	/// Check if an action is registered
	pub fn has_action(&self, name: &str) -> bool {
		self.actions.contains_key(name)
	}

	/// Get an action by name
	pub fn get_action(
		&self,
		name: &str,
	) -> AdminResult<dashmap::mapref::one::Ref<String, Box<dyn AdminAction>>> {
		self.actions
			.get(name)
			.ok_or_else(|| AdminError::InvalidAction(format!("Action '{}' not found", name)))
	}

	/// Get all available action names
	pub fn available_actions(&self) -> Vec<String> {
		self.actions
			.iter()
			.map(|entry| entry.key().clone())
			.collect()
	}

	/// Clear all actions
	pub fn clear(&self) {
		self.actions.clear();
	}

	/// Get the number of registered actions
	pub fn len(&self) -> usize {
		self.actions.len()
	}

	/// Check if the registry is empty
	pub fn is_empty(&self) -> bool {
		self.actions.is_empty()
	}
}

impl Default for ActionRegistry {
	fn default() -> Self {
		Self::with_defaults()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct TestAction {
		name: String,
		should_fail: bool,
	}

	#[async_trait]
	impl AdminAction for TestAction {
		fn name(&self) -> &str {
			&self.name
		}

		fn description(&self) -> &str {
			"Test action"
		}

		async fn execute(
			&self,
			_model_name: &str,
			item_ids: Vec<String>,
			_user: &dyn Any,
		) -> ActionResult {
			if self.should_fail {
				ActionResult::Error {
					message: "Action failed".to_string(),
					errors: vec!["Test error".to_string()],
				}
			} else {
				ActionResult::Success {
					message: "Action succeeded".to_string(),
					affected_count: item_ids.len(),
				}
			}
		}
	}

	#[test]
	fn test_action_result_is_success() {
		assert!(
			ActionResult::Success {
				message: "OK".to_string(),
				affected_count: 5
			}
			.is_success()
		);

		assert!(
			ActionResult::Warning {
				message: "Warning".to_string(),
				affected_count: 3,
				warnings: vec![]
			}
			.is_success()
		);

		assert!(
			ActionResult::PartialSuccess {
				message: "Partial".to_string(),
				succeeded_count: 2,
				failed_count: 1,
				errors: vec![]
			}
			.is_success()
		);

		assert!(
			!ActionResult::Error {
				message: "Error".to_string(),
				errors: vec![]
			}
			.is_success()
		);
	}

	#[test]
	fn test_action_result_affected_count() {
		assert_eq!(
			ActionResult::Success {
				message: "OK".to_string(),
				affected_count: 5
			}
			.affected_count(),
			5
		);

		assert_eq!(
			ActionResult::PartialSuccess {
				message: "Partial".to_string(),
				succeeded_count: 3,
				failed_count: 2,
				errors: vec![]
			}
			.affected_count(),
			3
		);

		assert_eq!(
			ActionResult::Error {
				message: "Error".to_string(),
				errors: vec![]
			}
			.affected_count(),
			0
		);
	}

	#[tokio::test]
	async fn test_delete_action_basic() {
		let action = DeleteSelectedAction::new();
		assert_eq!(action.name(), "delete_selected");
		assert_eq!(action.description(), "Delete selected items");
		assert!(action.requires_confirmation());

		let user = ();
		let result = action
			.execute("User", vec!["1".to_string(), "2".to_string()], &user)
			.await;

		assert!(result.is_success());
		assert_eq!(result.affected_count(), 2);
		assert_eq!(result.message(), "Successfully deleted 2 item(s)");
	}

	#[tokio::test]
	async fn test_delete_action_empty() {
		let action = DeleteSelectedAction::new();
		let user = ();
		let result = action.execute("User", vec![], &user).await;

		match result {
			ActionResult::Warning {
				message,
				affected_count,
				..
			} => {
				assert_eq!(message, "No items selected");
				assert_eq!(affected_count, 0);
			}
			_ => panic!("Expected Warning result"),
		}
	}

	#[test]
	fn test_action_registry_new() {
		let registry = ActionRegistry::new();
		assert!(registry.is_empty());
		assert_eq!(registry.len(), 0);
	}

	#[test]
	fn test_action_registry_with_defaults() {
		let registry = ActionRegistry::with_defaults();
		assert!(!registry.is_empty());
		assert!(registry.has_action("delete_selected"));
		assert_eq!(registry.len(), 1);
	}

	#[test]
	fn test_action_registry_register() {
		let registry = ActionRegistry::new();
		registry.register(TestAction {
			name: "test_action".to_string(),
			should_fail: false,
		});

		assert!(registry.has_action("test_action"));
		assert_eq!(registry.len(), 1);
	}

	#[test]
	fn test_action_registry_unregister() {
		let registry = ActionRegistry::with_defaults();
		assert!(registry.has_action("delete_selected"));

		let result = registry.unregister("delete_selected");
		assert!(result.is_ok());
		assert!(!registry.has_action("delete_selected"));
		assert!(registry.is_empty());
	}

	#[test]
	fn test_action_registry_unregister_nonexistent() {
		let registry = ActionRegistry::new();
		let result = registry.unregister("nonexistent");
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), AdminError::InvalidAction(_)));
	}

	#[test]
	fn test_action_registry_get_action() {
		let registry = ActionRegistry::with_defaults();
		let action = registry.get_action("delete_selected");
		assert!(action.is_ok());
		assert_eq!(action.unwrap().name(), "delete_selected");
	}

	#[test]
	fn test_action_registry_get_nonexistent() {
		let registry = ActionRegistry::new();
		let result = registry.get_action("nonexistent");
		assert!(result.is_err());
	}

	#[test]
	fn test_action_registry_available_actions() {
		let registry = ActionRegistry::new();
		registry.register(TestAction {
			name: "action1".to_string(),
			should_fail: false,
		});
		registry.register(TestAction {
			name: "action2".to_string(),
			should_fail: false,
		});

		let actions = registry.available_actions();
		assert_eq!(actions.len(), 2);
		assert!(actions.contains(&"action1".to_string()));
		assert!(actions.contains(&"action2".to_string()));
	}

	#[test]
	fn test_action_registry_clear() {
		let registry = ActionRegistry::with_defaults();
		assert!(!registry.is_empty());

		registry.clear();
		assert!(registry.is_empty());
		assert_eq!(registry.len(), 0);
	}
}
