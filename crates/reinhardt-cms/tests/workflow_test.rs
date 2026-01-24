//! Tests for workflow engine

use reinhardt_cms::workflow::{PageState, WorkflowEngine, WorkflowTransition};
use rstest::rstest;
use uuid::Uuid;

#[rstest]
#[tokio::test]
async fn test_workflow_state_retrieval() {
	let engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();

	// New pages should be in Draft state
	let state = engine.get_state(page_id).await.unwrap();
	assert_eq!(state, PageState::Draft);
}

#[rstest]
#[tokio::test]
async fn test_workflow_valid_transition() {
	let mut engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Draft -> InReview
	let new_state = engine
		.transition(page_id, WorkflowTransition::SubmitForReview, user_id)
		.await
		.unwrap();
	assert_eq!(new_state, PageState::InReview);

	// InReview -> Approved
	let new_state = engine
		.transition(page_id, WorkflowTransition::Approve, user_id)
		.await
		.unwrap();
	assert_eq!(new_state, PageState::Approved);

	// Approved -> Published
	let new_state = engine
		.transition(page_id, WorkflowTransition::Publish, user_id)
		.await
		.unwrap();
	assert_eq!(new_state, PageState::Published);
}

#[rstest]
#[tokio::test]
async fn test_workflow_invalid_transition() {
	let mut engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Draft -> Publish (invalid, should go through review first)
	let result = engine
		.transition(page_id, WorkflowTransition::Publish, user_id)
		.await;
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn test_version_creation() {
	let mut engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();
	let author_id = Uuid::new_v4();

	// Create first version
	let content = serde_json::json!({"title": "Test Page", "content": "Hello"});
	let version1 = engine
		.create_version(
			page_id,
			author_id,
			content,
			Some("Initial version".to_string()),
		)
		.await
		.unwrap();

	assert_eq!(version1.page_id, page_id);
	assert_eq!(version1.version_number, 1);
	assert_eq!(version1.author_id, author_id);

	// Create second version
	let content = serde_json::json!({"title": "Test Page", "content": "Updated"});
	let version2 = engine
		.create_version(
			page_id,
			author_id,
			content,
			Some("Updated content".to_string()),
		)
		.await
		.unwrap();

	assert_eq!(version2.version_number, 2);
}

#[rstest]
#[tokio::test]
async fn test_version_history() {
	let mut engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();
	let author_id = Uuid::new_v4();

	// Create multiple versions
	for i in 1..=3 {
		let content = serde_json::json!({"version": i});
		engine
			.create_version(page_id, author_id, content, Some(format!("Version {}", i)))
			.await
			.unwrap();
	}

	// Get version history
	let versions = engine.get_versions(page_id).await.unwrap();
	assert_eq!(versions.len(), 3);
	assert_eq!(versions[0].version_number, 1);
	assert_eq!(versions[1].version_number, 2);
	assert_eq!(versions[2].version_number, 3);
}

#[rstest]
#[tokio::test]
async fn test_version_restoration() {
	let mut engine = WorkflowEngine::new();

	let page_id = Uuid::new_v4();
	let author_id = Uuid::new_v4();

	// Create versions
	let content1 = serde_json::json!({"version": 1});
	let version1 = engine
		.create_version(page_id, author_id, content1, Some("Version 1".to_string()))
		.await
		.unwrap();

	let content2 = serde_json::json!({"version": 2});
	engine
		.create_version(page_id, author_id, content2, Some("Version 2".to_string()))
		.await
		.unwrap();

	// Restore to version 1
	let restored = engine
		.restore_version(page_id, version1.id, author_id)
		.await
		.unwrap();

	assert_eq!(restored.content["version"], 1);
	assert_eq!(restored.version_number, 3); // New version number
}
