//! Integration tests for Messages Framework
//!
//! These tests verify that reinhardt-messages works correctly with different storage backends.

use reinhardt_messages::{
    CookieStorage, FallbackStorage, Level, MemoryStorage, Message, MessageStorage, SessionStorage,
};

// ============================================================================
// Basic Message Operations Tests
// ============================================================================

#[test]
fn test_add_message() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::info("Welcome to the site!"));

    let messages = storage.peek();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "Welcome to the site!");
    assert_eq!(messages[0].level, Level::Info);
}

#[test]
fn test_get_messages() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::success("Profile updated successfully"));
    storage.add(Message::info("Check your email for confirmation"));

    let messages = storage.get_all();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "Profile updated successfully");
    assert_eq!(messages[1].text, "Check your email for confirmation");
}

#[test]
fn test_messages_consumed_after_read() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::info("This message will be consumed"));

    // First read - messages retrieved and consumed
    let messages_first = storage.get_all();
    assert_eq!(messages_first.len(), 1);

    // Second read - no messages (already consumed)
    let messages_second = storage.get_all();
    assert_eq!(messages_second.len(), 0);
}

// ============================================================================
// Message Levels Tests
// ============================================================================

#[test]
fn test_message_levels() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::debug("Debug: SQL query executed"));
    storage.add(Message::info("Info: User logged in"));
    storage.add(Message::success("Success: Form submitted"));
    storage.add(Message::warning("Warning: Disk space low"));
    storage.add(Message::error("Error: Failed to save"));

    let messages = storage.get_all();
    assert_eq!(messages.len(), 5);

    assert_eq!(messages[0].level, Level::Debug);
    assert_eq!(messages[1].level, Level::Info);
    assert_eq!(messages[2].level, Level::Success);
    assert_eq!(messages[3].level, Level::Warning);
    assert_eq!(messages[4].level, Level::Error);
}

#[test]
fn test_message_level_filtering() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::debug("Debug message"));
    storage.add(Message::info("Info message"));
    storage.add(Message::warning("Warning message"));
    storage.add(Message::error("Error message"));

    let messages = storage.get_all();

    // Filter messages with level >= Warning
    let important_messages: Vec<_> = messages
        .into_iter()
        .filter(|m| m.level >= Level::Warning)
        .collect();

    assert_eq!(important_messages.len(), 2);
    assert_eq!(important_messages[0].level, Level::Warning);
    assert_eq!(important_messages[1].level, Level::Error);
}

#[test]
fn test_message_tags() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::info("Info message"));
    storage.add(Message::error("Error message"));

    let messages = storage.get_all();

    // Each message has a default tag matching its level
    assert!(messages[0].tags().contains(&"info".to_string()));
    assert!(messages[1].tags().contains(&"error".to_string()));
}

#[test]
fn test_custom_message_tags() {
    let mut storage = MemoryStorage::new();

    let msg = Message::success("Data imported successfully")
        .with_tags(vec!["import".to_string(), "data".to_string()]);
    storage.add(msg);

    let messages = storage.get_all();
    let tags = messages[0].tags();

    assert!(tags.contains(&"success".to_string())); // Default level tag
    assert!(tags.contains(&"import".to_string())); // Custom tag
    assert!(tags.contains(&"data".to_string())); // Custom tag
}

// ============================================================================
// Message Storage Backend Tests
// ============================================================================

#[test]
fn test_session_storage_backend() {
    let mut storage = SessionStorage::new();

    storage.add(Message::info("Session message 1"));
    storage.add(Message::warning("Session message 2"));

    let messages = storage.get_all();
    assert_eq!(messages.len(), 2);

    // Messages consumed after retrieval
    let empty_messages = storage.get_all();
    assert_eq!(empty_messages.len(), 0);
}

#[test]
fn test_cookie_storage_backend() {
    let mut storage = CookieStorage::new();

    storage.add(Message::success("Cookie message"));

    let messages = storage.peek();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "Cookie message");
}

#[test]
fn test_fallback_storage_backend() {
    let mut storage = FallbackStorage::new();

    // FallbackStorage tries SessionStorage first, falls back to CookieStorage
    storage.add(Message::info("Fallback message"));

    let messages = storage.get_all();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "Fallback message");
}

// ============================================================================
// Message Context Tests (Template Integration)
// ============================================================================

#[test]
fn test_messages_in_template_context() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::success("Form submitted successfully"));
    storage.add(Message::info("Check your email"));

    // Simulate retrieving messages for template rendering
    let messages_for_template = storage.get_all();

    assert_eq!(messages_for_template.len(), 2);

    // Messages can be serialized for templates
    for msg in &messages_for_template {
        assert!(!msg.text.is_empty());
        assert!(!msg.tags().is_empty());
    }
}

#[test]
fn test_extra_message_data() {
    let mut storage = MemoryStorage::new();

    let mut msg = Message::warning("Disk space low");
    msg.add_tag("system".to_string());
    msg.add_tag("alert".to_string());

    storage.add(msg);

    let messages = storage.get_all();
    let tags = messages[0].tags();

    // Should have level tag + extra tags
    assert!(tags.contains(&"warning".to_string()));
    assert!(tags.contains(&"system".to_string()));
    assert!(tags.contains(&"alert".to_string()));
    assert_eq!(tags.len(), 3);
}

// ============================================================================
// Storage Persistence Tests
// ============================================================================

#[test]
fn test_peek_does_not_consume() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::info("Message 1"));
    storage.add(Message::info("Message 2"));

    // Peek multiple times
    let peek1 = storage.peek();
    let peek2 = storage.peek();

    assert_eq!(peek1.len(), 2);
    assert_eq!(peek2.len(), 2);

    // Get all finally consumes
    let messages = storage.get_all();
    assert_eq!(messages.len(), 2);

    // Now peek returns empty
    let peek3 = storage.peek();
    assert_eq!(peek3.len(), 0);
}

#[test]
fn test_clear_messages() {
    let mut storage = MemoryStorage::new();

    storage.add(Message::info("Message 1"));
    storage.add(Message::info("Message 2"));
    storage.add(Message::info("Message 3"));

    assert_eq!(storage.peek().len(), 3);

    storage.clear();

    assert_eq!(storage.peek().len(), 0);
}
