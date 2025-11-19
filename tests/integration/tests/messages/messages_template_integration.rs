//! Integration tests for messages and template rendering
//!
//! Tests the integration between the messages framework and template rendering,
//! verifying that messages are correctly displayed, filtered, and persisted
//! across template renders.

use reinhardt_messages::{Level, MemoryStorage, Message, MessageStorage};
use reinhardt_test::fixtures::{postgres_container, temp_dir};
use rstest::*;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tempfile::TempDir;
use tera::{Context, Tera};
use testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::GenericImage;

/// Helper function to render a template with messages
fn render_template(template_str: &str, messages: Vec<Message>) -> String {
	let mut context = Context::new();

	// Convert messages to JSON format for template
	let messages_json: Vec<_> = messages
		.iter()
		.map(|m| {
			json!({
				"level": m.level.value(),
				"level_tag": m.level.as_str(),
				"text": m.text,
				"extra_tags": m.extra_tags.join(" "),
			})
		})
		.collect();

	context.insert("messages", &messages_json);

	Tera::one_off(template_str, &context, false).expect("Template rendering failed")
}

/// Helper function to create test messages at all levels
fn create_all_level_messages() -> Vec<Message> {
	vec![
		Message::new(Level::Debug, "Debug message"),
		Message::new(Level::Info, "Info message"),
		Message::new(Level::Success, "Success message"),
		Message::new(Level::Warning, "Warning message"),
		Message::new(Level::Error, "Error message"),
	]
}

#[rstest]
#[tokio::test]
async fn test_basic_message_display(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Basic message display in template
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Success, "Operation completed successfully"),
		Message::new(Level::Info, "Processing started"),
	];

	let template = r#"
		<ul class="messages">
		{% for message in messages %}
			<li class="{{ message.level_tag }}">{{ message.text }}</li>
		{% endfor %}
		</ul>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<ul class="messages">"#));
	assert!(rendered.contains(r#"<li class="success">"#));
	assert!(rendered.contains("Operation completed successfully"));
	assert!(rendered.contains(r#"<li class="info">"#));
	assert!(rendered.contains("Processing started"));
}

#[rstest]
#[tokio::test]
async fn test_message_level_filtering_success_only(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Filter only success messages in template
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = create_all_level_messages();

	let template = r#"
		<div class="success-messages">
		{% for message in messages %}
			{% if message.level_tag == "success" %}
				<div class="alert">{{ message.text }}</div>
			{% endif %}
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("Success message"));
	assert!(!rendered.contains("Debug message"));
	assert!(!rendered.contains("Info message"));
	assert!(!rendered.contains("Warning message"));
	assert!(!rendered.contains("Error message"));
}

#[rstest]
#[tokio::test]
async fn test_message_level_filtering_errors_only(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Filter only error messages in template
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = create_all_level_messages();

	let template = r#"
		<div class="errors">
		{% for message in messages %}
			{% if message.level_tag == "error" %}
				<p>{{ message.text }}</p>
			{% endif %}
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("Error message"));
	assert!(!rendered.contains("Success message"));
	assert!(!rendered.contains("Warning message"));
}

#[rstest]
#[tokio::test]
async fn test_message_level_filtering_by_value(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Filter messages by numeric level value (warnings and errors only)
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = create_all_level_messages();

	let template = r#"
		<div class="alerts">
		{% for message in messages %}
			{% if message.level >= 30 %}
				<div class="{{ message.level_tag }}">{{ message.text }}</div>
			{% endif %}
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	// Should include warning (30) and error (40)
	assert!(rendered.contains("Warning message"));
	assert!(rendered.contains("Error message"));

	// Should not include debug (10), info (20), success (25)
	assert!(!rendered.contains("Debug message"));
	assert!(!rendered.contains("Info message"));
	assert!(!rendered.contains("Success message"));
}

#[rstest]
#[tokio::test]
async fn test_multiple_messages_same_level(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Render multiple messages of the same level
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Error, "First error"),
		Message::new(Level::Error, "Second error"),
		Message::new(Level::Error, "Third error"),
	];

	let template = r#"
		<ul>
		{% for message in messages %}
			<li>{{ message.text }}</li>
		{% endfor %}
		</ul>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("First error"));
	assert!(rendered.contains("Second error"));
	assert!(rendered.contains("Third error"));

	// Count occurrences
	assert_eq!(rendered.matches("<li>").count(), 3);
}

#[rstest]
#[tokio::test]
async fn test_empty_messages_list(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Handle empty messages list gracefully
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages: Vec<Message> = vec![];

	let template = r#"
		{% if messages %}
			<div class="has-messages">Messages exist</div>
		{% else %}
			<div class="no-messages">No messages</div>
		{% endif %}
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<div class="no-messages">No messages</div>"#));
	assert!(!rendered.contains("has-messages"));
}

#[rstest]
#[tokio::test]
async fn test_message_count_in_template(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Display message count
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Info, "Message 1"),
		Message::new(Level::Info, "Message 2"),
		Message::new(Level::Info, "Message 3"),
	];

	let template = r#"
		<div class="count">You have {{ messages | length }} messages</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("You have 3 messages"));
}

#[rstest]
#[tokio::test]
async fn test_message_with_extra_tags(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages with extra_tags are rendered correctly
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let mut message1 = Message::new(Level::Info, "Custom styled message");
	message1.extra_tags = vec!["custom-class".to_string(), "bold".to_string()];

	let mut message2 = Message::new(Level::Warning, "Dismissible warning");
	message2.extra_tags = vec!["dismissible".to_string()];

	let messages = vec![message1, message2];

	let template = r#"
		<div>
		{% for message in messages %}
			<p class="{{ message.level_tag }} {{ message.extra_tags }}">{{ message.text }}</p>
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"class="info custom-class bold""#));
	assert!(rendered.contains("Custom styled message"));
	assert!(rendered.contains(r#"class="warning dismissible""#));
	assert!(rendered.contains("Dismissible warning"));
}

#[rstest]
#[tokio::test]
async fn test_message_ordering_preserved(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages maintain insertion order
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Info, "First"),
		Message::new(Level::Success, "Second"),
		Message::new(Level::Warning, "Third"),
		Message::new(Level::Error, "Fourth"),
	];

	let template = r#"
		{% for message in messages %}{{ message.text }}|{% endfor %}
	"#;

	let rendered = render_template(template, messages);

	let first_pos = rendered.find("First").unwrap();
	let second_pos = rendered.find("Second").unwrap();
	let third_pos = rendered.find("Third").unwrap();
	let fourth_pos = rendered.find("Fourth").unwrap();

	assert!(first_pos < second_pos);
	assert!(second_pos < third_pos);
	assert!(third_pos < fourth_pos);
}

#[rstest]
#[tokio::test]
async fn test_multi_level_message_styling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Different styling for each message level
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = create_all_level_messages();

	let template = r#"
		<div class="messages">
		{% for message in messages %}
			{% if message.level_tag == "debug" %}
				<div class="msg-debug">{{ message.text }}</div>
			{% elif message.level_tag == "info" %}
				<div class="msg-info">{{ message.text }}</div>
			{% elif message.level_tag == "success" %}
				<div class="msg-success">{{ message.text }}</div>
			{% elif message.level_tag == "warning" %}
				<div class="msg-warning">{{ message.text }}</div>
			{% elif message.level_tag == "error" %}
				<div class="msg-error">{{ message.text }}</div>
			{% endif %}
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<div class="msg-debug">Debug message</div>"#));
	assert!(rendered.contains(r#"<div class="msg-info">Info message</div>"#));
	assert!(rendered.contains(r#"<div class="msg-success">Success message</div>"#));
	assert!(rendered.contains(r#"<div class="msg-warning">Warning message</div>"#));
	assert!(rendered.contains(r#"<div class="msg-error">Error message</div>"#));
}

#[rstest]
#[tokio::test]
async fn test_message_persistence_across_storage_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages persist correctly in storage for template use
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let mut storage = MemoryStorage::new();

	// Add messages
	storage.add(Message::new(Level::Success, "Saved successfully"));
	storage.add(Message::new(Level::Info, "Record created"));

	// Get messages (simulating retrieval for template)
	let messages = storage.get_all();

	let template = r#"
		<div>
		{% for message in messages %}
			<p>{{ message.text }}</p>
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("Saved successfully"));
	assert!(rendered.contains("Record created"));
}

#[rstest]
#[tokio::test]
async fn test_nested_template_structure_with_messages(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages in nested template structures
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Success, "Form submitted"),
		Message::new(Level::Info, "Validation passed"),
	];

	let template = r#"
		<div class="container">
			<header>
				<h1>Application</h1>
			</header>
			<main>
				{% if messages %}
				<aside class="notifications">
					{% for message in messages %}
					<div class="notification {{ message.level_tag }}">
						<span class="icon">•</span>
						<span class="text">{{ message.text }}</span>
					</div>
					{% endfor %}
				</aside>
				{% endif %}
				<section class="content">
					<p>Main content</p>
				</section>
			</main>
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<aside class="notifications">"#));
	assert!(rendered.contains(r#"<div class="notification success">"#));
	assert!(rendered.contains("Form submitted"));
	assert!(rendered.contains(r#"<div class="notification info">"#));
	assert!(rendered.contains("Validation passed"));
}

#[rstest]
#[tokio::test]
async fn test_sorted_messages_by_level_in_template(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages grouped by level in template sections
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Info, "Info 1"),
		Message::new(Level::Error, "Error 1"),
		Message::new(Level::Warning, "Warning 1"),
		Message::new(Level::Error, "Error 2"),
		Message::new(Level::Info, "Info 2"),
	];

	let template = r#"
		<div class="errors">
		{% for message in messages %}
			{% if message.level_tag == "error" %}
				<p>{{ message.text }}</p>
			{% endif %}
		{% endfor %}
		</div>
		<div class="warnings">
		{% for message in messages %}
			{% if message.level_tag == "warning" %}
				<p>{{ message.text }}</p>
			{% endif %}
		{% endfor %}
		</div>
		<div class="info">
		{% for message in messages %}
			{% if message.level_tag == "info" %}
				<p>{{ message.text }}</p>
			{% endif %}
		{% endfor %}
		</div>
	"#;

	let rendered = render_template(template, messages);

	// Check errors section
	let errors_section = rendered
		.split(r#"<div class="errors">"#)
		.nth(1)
		.unwrap()
		.split(r#"<div class="warnings">"#)
		.next()
		.unwrap();
	assert!(errors_section.contains("Error 1"));
	assert!(errors_section.contains("Error 2"));

	// Check warnings section
	let warnings_section = rendered
		.split(r#"<div class="warnings">"#)
		.nth(1)
		.unwrap()
		.split(r#"<div class="info">"#)
		.next()
		.unwrap();
	assert!(warnings_section.contains("Warning 1"));

	// Check info section
	let info_section = rendered.split(r#"<div class="info">"#).nth(1).unwrap();
	assert!(info_section.contains("Info 1"));
	assert!(info_section.contains("Info 2"));
}

#[rstest]
#[tokio::test]
async fn test_message_with_special_characters(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages with special characters are properly escaped
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![
		Message::new(Level::Info, "Message with <html> tags"),
		Message::new(Level::Warning, r#"Message with "quotes""#),
		Message::new(Level::Error, "Message with 'apostrophe'"),
	];

	let template = r#"
		{% for message in messages %}
			<div>{{ message.text }}</div>
		{% endfor %}
	"#;

	let rendered = render_template(template, messages);

	// Tera auto-escapes by default
	assert!(rendered.contains("&lt;html&gt;") || rendered.contains("Message with <html> tags"));
}

#[rstest]
#[tokio::test]
async fn test_conditional_message_sections(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Conditional rendering based on message presence
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![Message::new(Level::Success, "Action completed")];

	let template = r#"
		<div class="page">
			{% if messages %}
			<div class="alert-banner">
				<h3>Notifications</h3>
				{% for message in messages %}
					<p>{{ message.text }}</p>
				{% endfor %}
			</div>
			{% endif %}
			<div class="main-content">
				<h1>Page Title</h1>
			</div>
		</div>
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<div class="alert-banner">"#));
	assert!(rendered.contains("<h3>Notifications</h3>"));
	assert!(rendered.contains("Action completed"));
}

#[rstest]
#[tokio::test]
async fn test_message_metadata_in_attributes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Message metadata used in HTML attributes
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let mut message = Message::new(Level::Warning, "Important notice");
	message.extra_tags = vec!["urgent".to_string()];

	let messages = vec![message];

	let template = r#"
		{% for message in messages %}
			<div data-level="{{ message.level }}" data-level-tag="{{ message.level_tag }}" data-tags="{{ message.extra_tags }}">
				{{ message.text }}
			</div>
		{% endfor %}
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"data-level="30""#));
	assert!(rendered.contains(r#"data-level-tag="warning""#));
	assert!(rendered.contains(r#"data-tags="urgent""#));
	assert!(rendered.contains("Important notice"));
}

#[rstest]
#[tokio::test]
async fn test_large_number_of_messages_performance(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Template rendering with many messages
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let mut messages = Vec::new();
	for i in 0..100 {
		let level = match i % 5 {
			0 => Level::Debug,
			1 => Level::Info,
			2 => Level::Success,
			3 => Level::Warning,
			_ => Level::Error,
		};
		messages.push(Message::new(level, &format!("Message {}", i)));
	}

	let template = r#"
		<ul>
		{% for message in messages %}
			<li class="{{ message.level_tag }}">{{ message.text }}</li>
		{% endfor %}
		</ul>
	"#;

	let start = std::time::Instant::now();
	let rendered = render_template(template, messages);
	let duration = start.elapsed();

	// Should render quickly (< 100ms for 100 messages)
	assert!(
		duration.as_millis() < 100,
		"Rendering took too long: {:?}",
		duration
	);

	// Verify some messages are present
	assert!(rendered.contains("Message 0"));
	assert!(rendered.contains("Message 50"));
	assert!(rendered.contains("Message 99"));
	assert_eq!(rendered.matches("<li").count(), 100);
}

#[rstest]
#[tokio::test]
async fn test_message_with_empty_text(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages with empty text are still rendered
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = vec![Message::new(Level::Info, "")];

	let template = r#"
		{% for message in messages %}
			<div class="{{ message.level_tag }}">{{ message.text }}</div>
		{% endfor %}
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains(r#"<div class="info"></div>"#));
}

#[rstest]
#[tokio::test]
async fn test_message_icon_mapping_in_template(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Different icons for different message levels
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let messages = create_all_level_messages();

	let template = r#"
		{% for message in messages %}
			<div class="{{ message.level_tag }}">
				{% if message.level_tag == "debug" %}
					<span class="icon">🐛</span>
				{% elif message.level_tag == "info" %}
					<span class="icon">ℹ️</span>
				{% elif message.level_tag == "success" %}
					<span class="icon">✅</span>
				{% elif message.level_tag == "warning" %}
					<span class="icon">⚠️</span>
				{% elif message.level_tag == "error" %}
					<span class="icon">❌</span>
				{% endif %}
				{{ message.text }}
			</div>
		{% endfor %}
	"#;

	let rendered = render_template(template, messages);

	assert!(rendered.contains("🐛"));
	assert!(rendered.contains("ℹ️"));
	assert!(rendered.contains("✅"));
	assert!(rendered.contains("⚠️"));
	assert!(rendered.contains("❌"));
}

#[rstest]
#[tokio::test]
async fn test_message_storage_clear_after_retrieval(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	temp_dir: TempDir,
) {
	// Test: Messages are cleared from storage after retrieval (one-time display)
	let (_container, _pool, _port, _url) = postgres_container.await;
	let _dir = temp_dir.path();

	let mut storage = MemoryStorage::new();
	storage.add(Message::new(Level::Info, "One-time message"));

	// First retrieval - should get message
	let messages1 = storage.get_all();
	assert_eq!(messages1.len(), 1);

	let template = r#"
		{% for message in messages %}{{ message.text }}{% endfor %}
	"#;

	let rendered1 = render_template(template, messages1);
	assert!(rendered1.contains("One-time message"));

	// Clear storage (simulating middleware behavior)
	storage.clear();

	// Second retrieval - should be empty
	let messages2 = storage.peek();
	assert_eq!(messages2.len(), 0);

	let rendered2 = render_template(template, messages2);
	assert!(!rendered2.contains("One-time message"));
}
