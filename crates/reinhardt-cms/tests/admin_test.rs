//! Tests for admin UI components

use reinhardt_cms::admin::PageEditor;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

#[rstest]
#[tokio::test]
async fn test_render_edit_form_empty() {
	let editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	// Render form for a page without cached data
	let form_html = editor.render_edit_form(page_id).await.unwrap();

	// Verify form contains basic elements
	assert!(form_html.contains("<form"));
	assert!(form_html.contains("id=\"page-edit-form\""));
	assert!(form_html.contains("name=\"title\""));
	assert!(form_html.contains("name=\"slug\""));
	assert!(form_html.contains("name=\"content\""));
}

#[rstest]
#[tokio::test]
async fn test_render_edit_form_with_data() {
	let mut editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	// Save some page data
	let page_data = json!({
		"title": "Test Page",
		"slug": "test-page",
		"content": "Test content"
	});
	editor.save_page(page_id, page_data).await.unwrap();

	// Render form
	let form_html = editor.render_edit_form(page_id).await.unwrap();

	// Verify form contains the saved data
	assert!(form_html.contains("value=\"Test Page\""));
	assert!(form_html.contains("value=\"test-page\""));
	assert!(form_html.contains("Test content"));
}

#[rstest]
#[tokio::test]
async fn test_save_page_valid() {
	let mut editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	let page_data = json!({
		"title": "My Page",
		"slug": "my-page",
		"content": "Page content here"
	});

	// Save should succeed
	let result = editor.save_page(page_id, page_data.clone()).await;
	assert!(result.is_ok());

	// Verify data was saved by rendering form
	let form_html = editor.render_edit_form(page_id).await.unwrap();
	assert!(form_html.contains("value=\"My Page\""));
}

#[rstest]
#[tokio::test]
async fn test_save_page_missing_title() {
	let mut editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	let page_data = json!({
		"slug": "my-page",
		"content": "Page content here"
	});

	// Save should fail due to missing title
	let result = editor.save_page(page_id, page_data).await;
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn test_save_page_missing_slug() {
	let mut editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	let page_data = json!({
		"title": "My Page",
		"content": "Page content here"
	});

	// Save should fail due to missing slug
	let result = editor.save_page(page_id, page_data).await;
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn test_save_page_invalid_format() {
	let mut editor = PageEditor::new();
	let page_id = Uuid::new_v4();

	// Non-object data
	let page_data = json!(["invalid", "data"]);

	// Save should fail due to invalid format
	let result = editor.save_page(page_id, page_data).await;
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn test_multiple_pages() {
	let mut editor = PageEditor::new();

	let page1_id = Uuid::new_v4();
	let page2_id = Uuid::new_v4();

	// Save different data for two pages
	let page1_data = json!({
		"title": "Page 1",
		"slug": "page-1",
		"content": "Content 1"
	});

	let page2_data = json!({
		"title": "Page 2",
		"slug": "page-2",
		"content": "Content 2"
	});

	editor.save_page(page1_id, page1_data).await.unwrap();
	editor.save_page(page2_id, page2_data).await.unwrap();

	// Verify each page has its own data
	let form1 = editor.render_edit_form(page1_id).await.unwrap();
	let form2 = editor.render_edit_form(page2_id).await.unwrap();

	assert!(form1.contains("value=\"Page 1\""));
	assert!(form2.contains("value=\"Page 2\""));
}
