//! Integration tests for cookie-based message storage

#[cfg(test)]
mod tests {
	use reinhardt_core::messages::utils::{bisect_keep_left, bisect_keep_right};
	use reinhardt_core::messages::{CookieStorage, Level, Message, MessageStorage, SafeData};
	use rstest::rstest;

	#[rstest]
	fn test_cookie_storage_get() {
		// Test retrieving messages from cookie storage
		let mut storage = CookieStorage::new();

		// Add some messages
		storage.add(Message::new(Level::Info, "Test message 1"));
		storage.add(Message::new(Level::Success, "Test message 2"));

		let messages = storage.get_all();
		assert_eq!(messages.len(), 2);
		assert_eq!(messages[0].text, "Test message 1");
		assert_eq!(messages[1].text, "Test message 2");
	}

	#[rstest]
	fn test_cookie_settings() {
		// Test that CookieStorage can be configured with custom settings
		let storage = CookieStorage::new()
			.with_cookie_name("custom_messages")
			.with_max_size(2048);

		assert_eq!(storage.cookie_name(), "custom_messages");
	}

	#[rstest]
	fn test_get_bad_cookie() {
		// Test that invalid cookie data is handled gracefully
		let mut storage = CookieStorage::new();

		// Load invalid JSON - should handle gracefully
		let result = storage.load_from_cookie("invalid{json}data");
		assert!(result.is_ok());

		// Should return empty list for bad cookie
		assert_eq!(storage.get_all().len(), 0);
	}

	#[rstest]
	fn test_max_cookie_length() {
		// Test that older messages are removed when cookie size limit is exceeded
		let mut storage = CookieStorage::new().with_max_size(150);

		// Add messages that exceed max cookie size
		for i in 0..10 {
			storage.add(Message::new(
				Level::Info,
				format!("Long message number {}", i),
			));
		}

		// Update will drop oldest messages to fit size
		let unstored = storage.update();

		// Some messages should not fit
		assert!(!unstored.is_empty(), "Some messages should be unstored");

		// Get cookie value to verify remaining messages fit
		let (cookie_value, _) = storage.get_cookie_value().unwrap();
		assert!(
			cookie_value.len() <= 150,
			"Cookie value should fit within limit"
		);
	}

	#[rstest]
	fn test_message_rfc6265() {
		// Test that message encoding complies with RFC 6265
		let text = r#"Test with special chars: , ; \ ""#;
		let encoded = CookieStorage::encode_for_cookie(text);

		// RFC 6265 prohibits these characters
		assert!(!encoded.contains(","));
		assert!(!encoded.contains(";"));
		assert!(!encoded.contains("\\"));
		assert!(!encoded.contains("\""));
	}

	#[rstest]
	fn test_json_encoder_decoder() {
		// Test that messages are properly encoded/decoded
		let mut storage = CookieStorage::new();

		storage.add(Message::new(Level::Info, "Test message"));
		storage.add(
			Message::new(Level::Warning, "Warning message")
				.with_tags(vec!["important".to_string()]),
		);

		// Serialize
		let (cookie_value, unstored) = storage.get_cookie_value().unwrap();
		assert_eq!(unstored.len(), 0);

		// Deserialize
		let mut new_storage = CookieStorage::new();
		new_storage.load_from_cookie(&cookie_value).unwrap();

		let messages = new_storage.get_all();
		assert_eq!(messages.len(), 2);
		assert_eq!(messages[0].text, "Test message");
		assert_eq!(messages[1].text, "Warning message");
		assert!(messages[1].extra_tags.contains(&"important".to_string()));
	}

	#[rstest]
	fn test_safedata() {
		// Test that SafeData maintains its safe status through serialization
		let safe_html = SafeData::new("<b>Bold</b>");

		// Serialize and deserialize
		let json = serde_json::to_string(&safe_html).unwrap();
		let deserialized: SafeData = serde_json::from_str(&json).unwrap();

		// Safe status should be preserved
		assert_eq!(safe_html.as_str(), deserialized.as_str());
		assert_eq!(safe_html.as_str(), "<b>Bold</b>");
	}

	#[rstest]
	fn test_extra_tags_cookie() {
		// Test that extra_tags are preserved in cookie storage
		let mut storage = CookieStorage::new();

		storage.add(Message::new(Level::Info, "Message with no tags"));
		storage.add(
			Message::new(Level::Success, "Message with tags")
				.with_tags(vec!["tag1".to_string(), "tag2".to_string()]),
		);

		// Serialize and deserialize
		let (cookie_value, _) = storage.get_cookie_value().unwrap();
		let mut new_storage = CookieStorage::new();
		new_storage.load_from_cookie(&cookie_value).unwrap();

		let messages = new_storage.get_all();
		assert_eq!(messages.len(), 2);
		assert!(messages[0].extra_tags.is_empty());
		assert_eq!(messages[1].extra_tags.len(), 2);
	}

	#[rstest]
	fn test_bisect_keep_left() {
		// Test bisect_keep_left utility function
		let items = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];

		let serializer = |items: &[String]| serde_json::to_vec(items).unwrap();

		// All should fit in large size
		let count = bisect_keep_left(&items, 1000, serializer);
		assert_eq!(count, 3);

		// Only some should fit in small size
		let count = bisect_keep_left(&items, 20, serializer);
		assert!(count < 3);
	}

	#[rstest]
	fn test_bisect_keep_right() {
		// Test bisect_keep_right utility function
		let items = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];

		let serializer = |items: &[String]| serde_json::to_vec(items).unwrap();

		// All should fit in large size
		let count = bisect_keep_right(&items, 1000, serializer);
		assert_eq!(count, 3);

		// Only some should fit in small size
		let count = bisect_keep_right(&items, 20, serializer);
		assert!(count < 3);
	}
}
