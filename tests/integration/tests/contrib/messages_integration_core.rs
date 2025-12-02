//! Messages storage integration tests
//!
//! Integration tests for message storage functionality across reinhardt-messages.
//! These tests verify the interaction between multiple storage backends
//! (SessionStorage and CookieStorage) and message serialization.
//!
//! Based on Django's messages tests from:
//! - django/tests/messages_tests/test_session.py
//! - django/tests/messages_tests/test_cookie.py

use reinhardt_messages::{CookieStorage, Level, Message, MessageStorage, SessionStorage};

// ========== Session Storage Integration Tests ==========

#[test]
fn test_session_storage_add_and_retrieve() {
	let mut storage = SessionStorage::new();

	storage.add(Message::new(Level::Info, "Test message"));
	storage.add(Message::new(Level::Success, "Success!"));

	let messages = storage.get_all();
	assert_eq!(messages.len(), 2);
	assert_eq!(messages[0].text, "Test message");
	assert_eq!(messages[1].text, "Success!");
}

#[test]
fn test_session_storage_get_all_consumes_messages() {
	let mut storage = SessionStorage::new();

	storage.add(Message::new(Level::Info, "Message 1"));
	storage.add(Message::new(Level::Info, "Message 2"));

	let messages = storage.get_all();
	assert_eq!(messages.len(), 2);

	// After get_all, storage should be empty
	let messages_again = storage.get_all();
	assert_eq!(messages_again.len(), 0);
}

#[test]
fn test_session_storage_peek_does_not_consume() {
	let mut storage = SessionStorage::new();

	storage.add(Message::new(Level::Info, "Message"));

	// peek() should not consume messages
	let peeked = storage.peek();
	assert_eq!(peeked.len(), 1);

	// Messages should still be there
	let peeked_again = storage.peek();
	assert_eq!(peeked_again.len(), 1);

	// get_all() should still return the message
	let messages = storage.get_all();
	assert_eq!(messages.len(), 1);
}

#[test]
fn test_session_storage_clear() {
	let mut storage = SessionStorage::new();

	storage.add(Message::new(Level::Info, "Message"));
	storage.clear();

	let messages = storage.get_all();
	assert_eq!(messages.len(), 0);
}

#[test]
fn test_session_storage_custom_session_key() {
	let storage = SessionStorage::new().with_session_key("custom_messages");
	assert_eq!(storage.session_key(), "custom_messages");
}

// ========== Cookie Storage Integration Tests ==========

#[test]
fn test_cookie_storage_add_and_retrieve() {
	let mut storage = CookieStorage::new();

	storage.add(Message::new(Level::Success, "Saved!"));
	storage.add(Message::new(Level::Warning, "Warning!"));

	let messages = storage.get_all();
	assert_eq!(messages.len(), 2);
	assert_eq!(messages[0].text, "Saved!");
	assert_eq!(messages[1].text, "Warning!");
}

#[test]
fn test_cookie_storage_custom_cookie_name() {
	let storage = CookieStorage::new().with_cookie_name("my_messages");
	assert_eq!(storage.cookie_name(), "my_messages");
}

#[test]
fn test_cookie_storage_max_size() {
	let storage = CookieStorage::new().with_max_size(8192);
	assert_eq!(storage.max_cookie_size(), 8192);
}

#[test]
fn test_cookie_storage_serialize() {
	let mut storage = CookieStorage::new();

	storage.add(Message::new(Level::Info, "Cookie message"));

	let serialized = storage.serialize().expect("Failed to serialize");
	assert!(serialized.contains("Cookie message"));
	assert!(serialized.contains("Info"));
}

#[test]
fn test_cookie_storage_round_trip() {
	let mut storage1 = CookieStorage::new();

	storage1.add(Message::new(Level::Success, "Message 1"));
	storage1.add(Message::new(Level::Error, "Message 2"));

	// Serialize from first storage
	let (cookie_value, unstored) = storage1.get_cookie_value().unwrap();
	assert_eq!(unstored.len(), 0);

	// Deserialize into second storage
	let mut storage2 = CookieStorage::new();
	storage2.load_from_cookie(&cookie_value).unwrap();

	let messages = storage2.get_all();
	assert_eq!(messages.len(), 2);
	assert_eq!(messages[0].text, "Message 1");
	assert_eq!(messages[1].text, "Message 2");
}

#[test]
fn test_cookie_storage_size_limit() {
	let mut storage = CookieStorage::new().with_max_size(100);

	// Add messages that will exceed size limit
	storage.add(Message::new(Level::Info, "Short message"));
	storage.add(Message::new(
		Level::Info,
		"This is a very long message that will exceed the cookie size limit",
	));

	let unstored = storage.update();
	assert!(!unstored.is_empty(), "Some messages should be unstored");
}

#[test]
fn test_cookie_storage_encode_for_cookie() {
	// Test that RFC 6265 prohibited characters are properly encoded
	let text = r#"Test with special chars: , ; \ ""#;
	let encoded = CookieStorage::encode_for_cookie(text);

	// RFC 6265 prohibits these characters - they should be absent after encoding
	assert!(!encoded.contains(","));
	assert!(!encoded.contains(";"));
	assert!(!encoded.contains("\\"));
	assert!(!encoded.contains("\""));

	// Verify percent-encoding is used
	assert!(encoded.contains("%2C") || encoded.contains("%3B"));
}

// ========== Cross-Storage Integration Tests ==========

#[test]
fn test_message_transfer_between_storages() {
	// Create message in session storage
	let mut session_storage = SessionStorage::new();
	session_storage.add(Message::new(Level::Info, "Transfer test"));

	// Get messages from session
	let messages = session_storage.get_all();
	assert_eq!(messages.len(), 1);

	// Transfer to cookie storage
	let mut cookie_storage = CookieStorage::new();
	for msg in messages {
		cookie_storage.add(msg);
	}

	// Verify in cookie storage
	let cookie_messages = cookie_storage.peek();
	assert_eq!(cookie_messages.len(), 1);
	assert_eq!(cookie_messages[0].text, "Transfer test");
}

#[test]
fn test_message_with_tags_across_storages() {
	// Create tagged message
	let msg = Message::new(Level::Info, "Tagged message")
		.with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

	// Add to session storage
	let mut session_storage = SessionStorage::new();
	session_storage.add(msg.clone());

	// Serialize and transfer via cookie
	let mut cookie_storage = CookieStorage::new();
	cookie_storage.add(msg);
	let (cookie_value, _) = cookie_storage.get_cookie_value().unwrap();

	// Load from cookie into new storage
	let mut new_storage = CookieStorage::new();
	new_storage.load_from_cookie(&cookie_value).unwrap();

	let messages = new_storage.get_all();
	assert_eq!(messages.len(), 1);
	assert_eq!(messages[0].extra_tags.len(), 2);
	assert!(messages[0].extra_tags.contains(&"tag1".to_string()));
}

// ========== Message API Tests ==========

#[test]
fn test_message_level_tags() {
	let debug_msg = Message::new(Level::Debug, "Debug");
	assert_eq!(debug_msg.level.as_str(), "debug");

	let info_msg = Message::new(Level::Info, "Info");
	assert_eq!(info_msg.level.as_str(), "info");

	let success_msg = Message::new(Level::Success, "Success");
	assert_eq!(success_msg.level.as_str(), "success");

	let warning_msg = Message::new(Level::Warning, "Warning");
	assert_eq!(warning_msg.level.as_str(), "warning");

	let error_msg = Message::new(Level::Error, "Error");
	assert_eq!(error_msg.level.as_str(), "error");
}

#[test]
fn test_message_tags_method() {
	let msg =
		Message::new(Level::Info, "Tagged").with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

	let all_tags = msg.tags();
	assert_eq!(all_tags.len(), 3); // Level tag + 2 custom tags
	assert_eq!(all_tags[0], "info");
	assert!(all_tags.contains(&"tag1".to_string()));
	assert!(all_tags.contains(&"tag2".to_string()));
}

#[test]
fn test_message_add_tag_mutably() {
	let mut msg = Message::new(Level::Info, "Message");
	msg.add_tag("custom".to_string());
	msg.add_tag("another".to_string());

	assert_eq!(msg.extra_tags.len(), 2);
	assert!(msg.extra_tags.contains(&"custom".to_string()));
	assert!(msg.extra_tags.contains(&"another".to_string()));
}

// ========== Empty State Tests ==========

#[test]
fn test_empty_session_storage() {
	let mut storage = SessionStorage::new();
	let messages = storage.get_all();
	assert_eq!(messages.len(), 0);
}

#[test]
fn test_empty_cookie_storage() {
	let mut storage = CookieStorage::new();
	let messages = storage.get_all();
	assert_eq!(messages.len(), 0);
}

#[test]
fn test_empty_cookie_serialization() {
	let storage = CookieStorage::new();
	let serialized = storage.serialize().unwrap();
	assert_eq!(serialized, "[]");
}
