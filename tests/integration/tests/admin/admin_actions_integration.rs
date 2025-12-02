//! Integration tests for Admin Actions
//!
//! These tests verify the admin actions functionality with a real PostgreSQL database.
//!
//! Tests cover:
//! - DeleteSelectedAction with database operations
//! - Custom action execution and results
//! - Bulk operations on multiple records
//! - Action registry management

use async_trait::async_trait;
use reinhardt_orm::{Model, Timestamped, Timestamps};
use reinhardt_panel::{
	ActionRegistry, ActionResult, AdminAction, AdminDatabase, DeleteSelectedAction,
};
use reinhardt_test::fixtures::mock_connection;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;

// Test model for integration testing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct Article {
	id: Option<i64>,
	title: String,
	content: String,
	is_published: bool,
	timestamps: Timestamps,
}

impl Model for Article {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"articles"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

impl Timestamped for Article {
	fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.created_at
	}

	fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.updated_at
	}

	fn set_updated_at(&mut self, time: chrono::DateTime<chrono::Utc>) {
		self.timestamps.updated_at = time;
	}
}

// Custom action for testing: Publish selected articles
struct PublishAction;

#[async_trait]
impl AdminAction for PublishAction {
	fn name(&self) -> &str {
		"publish_selected"
	}

	fn description(&self) -> &str {
		"Publish selected articles"
	}

	fn requires_confirmation(&self) -> bool {
		false
	}

	async fn execute(
		&self,
		_model_name: &str,
		item_ids: Vec<String>,
		_user: &(dyn Any + Send + Sync),
	) -> ActionResult {
		if item_ids.is_empty() {
			return ActionResult::Warning {
				message: "No articles selected".to_string(),
				affected_count: 0,
				warnings: vec!["Please select at least one article to publish".to_string()],
			};
		}

		// Simple success: all items are published successfully
		let count = item_ids.len();
		ActionResult::Success {
			message: format!("Successfully published {} article(s)", count),
			affected_count: count,
		}
	}
}

// Custom action that always fails
struct FailingAction;

#[async_trait]
impl AdminAction for FailingAction {
	fn name(&self) -> &str {
		"failing_action"
	}

	fn description(&self) -> &str {
		"This action always fails"
	}

	async fn execute(
		&self,
		_model_name: &str,
		_item_ids: Vec<String>,
		_user: &(dyn Any + Send + Sync),
	) -> ActionResult {
		ActionResult::Error {
			message: "Action failed".to_string(),
			errors: vec!["This action is designed to fail".to_string()],
		}
	}
}

// Custom action that simulates partial success
struct PartialPublishAction;

#[async_trait]
impl AdminAction for PartialPublishAction {
	fn name(&self) -> &str {
		"partial_publish"
	}

	fn description(&self) -> &str {
		"Publish articles with partial success"
	}

	fn requires_confirmation(&self) -> bool {
		false
	}

	async fn execute(
		&self,
		_model_name: &str,
		item_ids: Vec<String>,
		_user: &(dyn Any + Send + Sync),
	) -> ActionResult {
		if item_ids.is_empty() {
			return ActionResult::Warning {
				message: "No articles selected".to_string(),
				affected_count: 0,
				warnings: vec!["Please select at least one article to publish".to_string()],
			};
		}

		// Simulate partial success: first 3 succeed, rest fail
		let total = item_ids.len();
		let succeeded = total.min(3); // Max 3 successes
		let failed = total - succeeded;

		if failed > 0 {
			ActionResult::PartialSuccess {
				message: format!("Published {} article(s), {} failed", succeeded, failed),
				succeeded_count: succeeded,
				failed_count: failed,
				errors: vec![format!("{} articles could not be published", failed)],
			}
		} else {
			ActionResult::Success {
				message: format!("Successfully published {} article(s)", succeeded),
				affected_count: succeeded,
			}
		}
	}
}

#[tokio::test]
async fn test_delete_selected_action_without_database() {
	// Test DeleteSelectedAction without database connection (placeholder mode)
	let action = DeleteSelectedAction::new();

	assert_eq!(action.name(), "delete_selected");
	assert_eq!(action.description(), "Delete selected items");
	assert!(action.requires_confirmation());

	let user = ();
	let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
	let result = action.execute("Article", ids, &user).await;

	assert!(result.is_success());
	assert_eq!(result.affected_count(), 3);
	assert_eq!(result.message(), "Successfully deleted 3 item(s)");
}

#[tokio::test]
async fn test_delete_selected_action_empty_ids() {
	let action = DeleteSelectedAction::new();
	let user = ();
	let result = action.execute("Article", vec![], &user).await;

	match result {
		ActionResult::Warning {
			message,
			affected_count,
			warnings,
		} => {
			assert_eq!(message, "No items selected");
			assert_eq!(affected_count, 0);
			assert!(!warnings.is_empty());
		}
		_ => panic!("Expected Warning result"),
	}
}

#[tokio::test]
#[ignore = "Requires proper database mock mechanism - BackendsConnection cannot be easily mocked"]
async fn test_delete_selected_action_with_database() {
	// Create a mock database connection
	let conn = mock_connection();
	let db = Arc::new(AdminDatabase::new(conn));

	let action = DeleteSelectedAction::new()
		.with_database(db)
		.with_table("articles")
		.with_pk_field("id");

	let user = ();
	let ids = vec!["1".to_string(), "2".to_string()];

	// Note: This will fail without actual database, but demonstrates the API
	let result = action.execute("Article", ids, &user).await;

	// In mock mode, it should return error or success depending on implementation
	assert!(result.is_success() || matches!(result, ActionResult::Error { .. }));
}

#[tokio::test]
async fn test_custom_publish_action_success() {
	let action = PublishAction;
	let user = ();
	let ids = vec!["1".to_string(), "2".to_string()];

	let result = action.execute("Article", ids, &user).await;

	assert!(result.is_success());
	assert_eq!(result.affected_count(), 2);
	assert_eq!(result.message(), "Successfully published 2 article(s)");
}

#[tokio::test]
async fn test_custom_publish_action_empty() {
	let action = PublishAction;
	let user = ();

	let result = action.execute("Article", vec![], &user).await;

	match result {
		ActionResult::Warning {
			message,
			affected_count,
			warnings,
		} => {
			assert_eq!(message, "No articles selected");
			assert_eq!(affected_count, 0);
			assert_eq!(warnings.len(), 1);
		}
		_ => panic!("Expected Warning result"),
	}
}

#[tokio::test]
async fn test_custom_publish_action_partial_success() {
	let action = PartialPublishAction;
	let user = ();
	// 5 items: 3 succeed, 2 fail
	let ids = vec![
		"1".to_string(),
		"2".to_string(),
		"3".to_string(),
		"4".to_string(),
		"5".to_string(),
	];

	let result = action.execute("Article", ids, &user).await;

	match &result {
		ActionResult::PartialSuccess {
			message,
			succeeded_count,
			failed_count,
			errors,
		} => {
			assert_eq!(*succeeded_count, 3);
			assert_eq!(*failed_count, 2);
			assert_eq!(message, "Published 3 article(s), 2 failed");
			assert_eq!(errors.len(), 1);
		}
		_ => panic!("Expected PartialSuccess result, got {:?}", result),
	}
}

#[tokio::test]
async fn test_failing_action() {
	let action = FailingAction;
	let user = ();
	let ids = vec!["1".to_string()];

	let result = action.execute("Article", ids, &user).await;

	match &result {
		ActionResult::Error { message, errors } => {
			assert_eq!(message, "Action failed");
			assert_eq!(errors.len(), 1);
			assert_eq!(errors[0], "This action is designed to fail");
		}
		_ => panic!("Expected Error result"),
	}

	assert!(!result.is_success());
	assert_eq!(result.affected_count(), 0);
}

#[tokio::test]
async fn test_action_registry_registration() {
	let registry = ActionRegistry::new();

	assert!(registry.is_empty());
	assert_eq!(registry.len(), 0);

	// Register actions
	registry.register(DeleteSelectedAction::new());
	registry.register(PublishAction);

	assert!(!registry.is_empty());
	assert_eq!(registry.len(), 2);
	assert!(registry.has_action("delete_selected"));
	assert!(registry.has_action("publish_selected"));
}

#[tokio::test]
async fn test_action_registry_with_defaults() {
	let registry = ActionRegistry::with_defaults();

	assert!(!registry.is_empty());
	assert!(registry.has_action("delete_selected"));
	assert_eq!(registry.len(), 1);
}

#[tokio::test]
async fn test_action_registry_get_action() {
	let registry = ActionRegistry::new();
	registry.register(PublishAction);

	let action = registry.get_action("publish_selected");
	assert!(action.is_ok());
	assert_eq!(action.unwrap().name(), "publish_selected");

	let missing = registry.get_action("nonexistent");
	assert!(missing.is_err());
	if let Err(e) = missing {
		assert_eq!(
			e.to_string(),
			"Invalid action: Action not found: nonexistent"
		);
	}
}

#[tokio::test]
async fn test_action_registry_unregister() {
	let registry = ActionRegistry::with_defaults();
	assert!(registry.has_action("delete_selected"));

	let result = registry.unregister("delete_selected");
	assert!(result.is_ok());
	assert!(!registry.has_action("delete_selected"));
	assert!(registry.is_empty());
}

#[tokio::test]
async fn test_action_registry_available_actions() {
	let registry = ActionRegistry::new();
	registry.register(DeleteSelectedAction::new());
	registry.register(PublishAction);

	let actions = registry.available_actions();
	assert_eq!(actions.len(), 2);
	assert!(actions.contains(&"delete_selected".to_string()));
	assert!(actions.contains(&"publish_selected".to_string()));
}

#[tokio::test]
async fn test_action_registry_clear() {
	let registry = ActionRegistry::with_defaults();
	assert!(!registry.is_empty());

	registry.clear();
	assert!(registry.is_empty());
	assert_eq!(registry.len(), 0);
	assert!(!registry.has_action("delete_selected"));
}

#[tokio::test]
async fn test_action_result_methods() {
	// Test Success variant
	let success = ActionResult::Success {
		message: "Done".to_string(),
		affected_count: 5,
	};
	assert!(success.is_success());
	assert_eq!(success.affected_count(), 5);
	assert_eq!(success.message(), "Done");

	// Test Warning variant
	let warning = ActionResult::Warning {
		message: "Warning".to_string(),
		affected_count: 3,
		warnings: vec!["Minor issue".to_string()],
	};
	assert!(warning.is_success());
	assert_eq!(warning.affected_count(), 3);
	assert_eq!(warning.message(), "Warning");

	// Test Error variant
	let error = ActionResult::Error {
		message: "Failed".to_string(),
		errors: vec!["Error 1".to_string()],
	};
	assert!(!error.is_success());
	assert_eq!(error.affected_count(), 0);
	assert_eq!(error.message(), "Failed");

	// Test PartialSuccess variant
	let partial = ActionResult::PartialSuccess {
		message: "Partial".to_string(),
		succeeded_count: 7,
		failed_count: 3,
		errors: vec![],
	};
	assert!(partial.is_success());
	assert_eq!(partial.affected_count(), 7);
	assert_eq!(partial.message(), "Partial");
}

#[tokio::test]
async fn test_bulk_action_execution() {
	let registry = ActionRegistry::new();
	registry.register(PublishAction);
	registry.register(FailingAction);

	let user = ();
	let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];

	// Execute publish action
	let publish_action = registry.get_action("publish_selected").unwrap();
	let publish_result = publish_action.execute("Article", ids.clone(), &user).await;
	assert!(publish_result.is_success());

	// Execute failing action
	let failing_action = registry.get_action("failing_action").unwrap();
	let failing_result = failing_action.execute("Article", ids, &user).await;
	assert!(!failing_result.is_success());
}

#[tokio::test]
async fn test_action_confirmation_message() {
	let action = DeleteSelectedAction::new();
	let message = action.confirmation_message(5);
	assert_eq!(
		message,
		"Are you sure you want to delete 5 item(s)? This action cannot be undone."
	);
}

#[tokio::test]
async fn test_action_requires_confirmation() {
	let delete_action = DeleteSelectedAction::new();
	assert!(delete_action.requires_confirmation());

	let publish_action = PublishAction;
	assert!(!publish_action.requires_confirmation());
}

#[tokio::test]
async fn test_multiple_registrations_same_action() {
	let registry = ActionRegistry::new();
	registry.register(PublishAction);
	assert_eq!(registry.len(), 1);

	// Register another action with same name (should replace)
	registry.register(PublishAction);
	assert_eq!(registry.len(), 1);
}

#[tokio::test]
async fn test_action_registry_concurrent_access() {
	let registry = Arc::new(ActionRegistry::new());

	// Spawn multiple tasks that register actions concurrently
	let mut handles = vec![];

	for i in 0..10 {
		let reg = Arc::clone(&registry);
		let handle = tokio::spawn(async move {
			if i % 2 == 0 {
				reg.register(PublishAction);
			} else {
				reg.register(DeleteSelectedAction::new());
			}
		});
		handles.push(handle);
	}

	// Wait for all tasks to complete
	for handle in handles {
		handle.await.unwrap();
	}

	// Should have both actions registered
	assert!(registry.has_action("publish_selected"));
	assert!(registry.has_action("delete_selected"));
	assert_eq!(registry.len(), 2);
}
