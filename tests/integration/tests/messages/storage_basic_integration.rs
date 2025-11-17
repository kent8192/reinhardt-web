//! Basic integration tests for message storage
//!
//! These tests verify the basic functionality of reinhardt-messages storage backends
//! without requiring full HTTP/middleware integration.

#[cfg(test)]
mod tests {
	use reinhardt_messages::{Level, MemoryStorage, Message, MessageStorage};
	use std::str::FromStr;

	#[test]
	fn test_memory_storage_add_and_get() {
		// Test intent: Verify MemoryStorage adds message and peek() retrieves it
		// without clearing the storage
		// Not intent: Thread safety, persistence, multi-message behavior
		let mut storage = MemoryStorage::new();

		// Add a message
		storage.add(Message::new(Level::Info, "Test message"));

		// Get messages
		let messages = storage.peek();
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].text, "Test message");
		assert_eq!(messages[0].level, Level::Info);
	}

	#[test]
	fn test_memory_storage_get_all_clears() {
		let mut storage = MemoryStorage::new();

		storage.add(Message::new(Level::Info, "Message 1"));
		storage.add(Message::new(Level::Warning, "Message 2"));

		// get_all should return and clear messages
		let messages = storage.get_all();
		assert_eq!(messages.len(), 2);

		// Verify storage is now empty
		let remaining = storage.peek();
		assert_eq!(remaining.len(), 0);
	}

	#[test]
	fn test_memory_storage_peek_does_not_clear() {
		let mut storage = MemoryStorage::new();

		storage.add(Message::new(Level::Success, "Success message"));

		// peek should not clear messages
		let messages = storage.peek();
		assert_eq!(messages.len(), 1);

		// Messages should still be there
		let messages_again = storage.peek();
		assert_eq!(messages_again.len(), 1);
	}

	#[test]
	fn test_memory_storage_multiple_messages() {
		let mut storage = MemoryStorage::new();

		// Add messages at different levels
		storage.add(Message::debug("Debug message"));
		storage.add(Message::info("Info message"));
		storage.add(Message::success("Success message"));
		storage.add(Message::warning("Warning message"));
		storage.add(Message::error("Error message"));

		let messages = storage.peek();
		assert_eq!(messages.len(), 5);

		// Verify order is preserved
		assert_eq!(messages[0].text, "Debug message");
		assert_eq!(messages[0].level, Level::Debug);

		assert_eq!(messages[1].text, "Info message");
		assert_eq!(messages[1].level, Level::Info);

		assert_eq!(messages[2].text, "Success message");
		assert_eq!(messages[2].level, Level::Success);

		assert_eq!(messages[3].text, "Warning message");
		assert_eq!(messages[3].level, Level::Warning);

		assert_eq!(messages[4].text, "Error message");
		assert_eq!(messages[4].level, Level::Error);
	}

	#[test]
	fn test_memory_storage_clear() {
		let mut storage = MemoryStorage::new();

		storage.add(Message::info("Message 1"));
		storage.add(Message::info("Message 2"));

		assert_eq!(storage.peek().len(), 2);

		// Clear should remove all messages
		storage.clear();

		assert_eq!(storage.peek().len(), 0);
	}

	#[test]
	fn test_message_with_extra_tags() {
		let mut storage = MemoryStorage::new();

		let message = Message::new(Level::Info, "Tagged message")
			.with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

		storage.add(message);

		let messages = storage.peek();
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].extra_tags, vec!["tag1", "tag2"]);
	}

	#[test]
	fn test_messages_storage_add_tag() {
		let mut message = Message::new(Level::Warning, "Test");
		assert!(message.extra_tags.is_empty());

		message.add_tag("custom".to_string());
		assert_eq!(message.extra_tags.len(), 1);
		assert_eq!(message.extra_tags[0], "custom");
	}

	#[test]
	fn test_message_tags_includes_level() {
		let message =
			Message::new(Level::Error, "Error occurred").with_tags(vec!["critical".to_string()]);

		let tags = message.tags();

		// First tag should be the level
		assert_eq!(tags[0], "error");
		// Followed by extra tags
		assert_eq!(tags[1], "critical");
	}

	#[test]
	fn test_level_ordering() {
		// Verify that levels can be compared
		assert!(Level::Debug < Level::Info);
		assert!(Level::Info < Level::Success);
		assert!(Level::Success < Level::Warning);
		assert!(Level::Warning < Level::Error);
	}

	#[test]
	fn test_level_as_str() {
		assert_eq!(Level::Debug.as_str(), "debug");
		assert_eq!(Level::Info.as_str(), "info");
		assert_eq!(Level::Success.as_str(), "success");
		assert_eq!(Level::Warning.as_str(), "warning");
		assert_eq!(Level::Error.as_str(), "error");
	}

	#[test]
	fn test_messages_integration_level_from_str() {
		assert_eq!(Level::from_str("debug"), Ok(Level::Debug));
		assert_eq!(Level::from_str("info"), Ok(Level::Info));
		assert_eq!(Level::from_str("success"), Ok(Level::Success));
		assert_eq!(Level::from_str("warning"), Ok(Level::Warning));
		assert_eq!(Level::from_str("error"), Ok(Level::Error));

		// Case insensitive
		assert_eq!(Level::from_str("DEBUG"), Ok(Level::Debug));
		assert_eq!(Level::from_str("INFO"), Ok(Level::Info));

		// Invalid
		assert!(Level::from_str("invalid").is_err());
	}

	#[test]
	fn test_empty_storage() {
		let storage = MemoryStorage::new();
		assert_eq!(storage.peek().len(), 0);
	}

	#[test]
	fn test_message_shortcut_constructors() {
		let debug = Message::debug("Debug");
		assert_eq!(debug.level, Level::Debug);
		assert_eq!(debug.text, "Debug");

		let info = Message::info("Info");
		assert_eq!(info.level, Level::Info);

		let success = Message::success("Success");
		assert_eq!(success.level, Level::Success);

		let warning = Message::warning("Warning");
		assert_eq!(warning.level, Level::Warning);

		let error = Message::error("Error");
		assert_eq!(error.level, Level::Error);
	}
}
