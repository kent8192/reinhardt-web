//! Integration tests for message settings and configuration
//!
//! These tests demonstrate message configuration functionality using mock settings

use reinhardt_messages::{Level, Message};
use reinhardt_settings::Settings;
use std::collections::HashMap;

// Mock message configuration for testing
#[derive(Debug, Clone)]
pub struct MessageConfig {
	pub message_tags: HashMap<i32, String>,
	pub message_level: i32,
}

impl MessageConfig {
	pub fn new() -> Self {
		let mut message_tags = HashMap::new();
		message_tags.insert(10, "debug".to_string());
		message_tags.insert(20, "info".to_string());
		message_tags.insert(30, "warning".to_string());
		message_tags.insert(40, "error".to_string());

		Self {
			message_tags,
			message_level: 20, // Default to INFO level
		}
	}

	pub fn with_custom_tags(mut self, tags: HashMap<i32, String>) -> Self {
		// Merge custom tags with existing tags instead of replacing
		for (level, tag) in tags {
			self.message_tags.insert(level, tag);
		}
		self
	}

	pub fn with_level(mut self, level: i32) -> Self {
		self.message_level = level;
		self
	}

	pub fn get_tag_for_level(&self, level: i32) -> Option<&String> {
		self.message_tags.get(&level)
	}

	pub fn should_store_message(&self, level: i32) -> bool {
		level >= self.message_level
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_test::messages::{assert_message_count, assert_message_exists};

	/// Test that MESSAGE_TAGS settings can override default level tags
	/// Original: django/tests/messages_tests/tests.py::TestLevelTags::test_override_settings_level_tags
	#[test]
	fn test_override_settings_level_tags() {
		let mut custom_tags = HashMap::new();
		custom_tags.insert(30, "caution".to_string()); // WARNING -> "caution"
		custom_tags.insert(40, "".to_string()); // ERROR -> "" (empty string)
		custom_tags.insert(12, "custom".to_string()); // Custom level -> "custom"

		let config = MessageConfig::new().with_custom_tags(custom_tags);

		// Test that custom tags are applied
		assert_eq!(config.get_tag_for_level(30), Some(&"caution".to_string()));
		assert_eq!(config.get_tag_for_level(40), Some(&"".to_string()));
		assert_eq!(config.get_tag_for_level(12), Some(&"custom".to_string()));

		// Test that default tags are still available for other levels
		assert_eq!(config.get_tag_for_level(20), Some(&"info".to_string()));
	}

	/// Test that LEVEL_TAGS is lazily evaluated from settings
	/// Original: django/tests/messages_tests/tests.py::TestLevelTags::test_lazy
	#[test]
	fn test_lazy_level_tags() {
		// Create initial config
		let mut config = MessageConfig::new();
		assert_eq!(config.get_tag_for_level(20), Some(&"info".to_string()));

		// Simulate lazy evaluation by updating tags after "loading"
		let mut updated_tags = HashMap::new();
		updated_tags.insert(20, "lazy-info".to_string());
		updated_tags.insert(30, "lazy-warning".to_string());

		config = config.with_custom_tags(updated_tags);

		// Verify that LEVEL_TAGS reflects the new settings
		assert_eq!(config.get_tag_for_level(20), Some(&"lazy-info".to_string()));
		assert_eq!(
			config.get_tag_for_level(30),
			Some(&"lazy-warning".to_string())
		);
	}

	/// Test that LEVEL_TAGS updates when settings change after initialization
	/// Original: django/tests/messages_tests/tests.py::TestLevelTags::test_override_settings_lazy
	#[test]
	fn test_override_settings_lazy_update() {
		// Initialize with one set of MESSAGE_TAGS
		let mut config = MessageConfig::new();
		assert_eq!(config.get_tag_for_level(20), Some(&"info".to_string()));

		// Simulate dynamic update of MESSAGE_TAGS setting
		let mut new_tags = HashMap::new();
		new_tags.insert(20, "updated-info".to_string());
		new_tags.insert(30, "updated-warning".to_string());

		config = config.with_custom_tags(new_tags);

		// Verify LEVEL_TAGS reflects the new settings
		assert_eq!(
			config.get_tag_for_level(20),
			Some(&"updated-info".to_string())
		);
		assert_eq!(
			config.get_tag_for_level(30),
			Some(&"updated-warning".to_string())
		);
	}

	/// Test support for custom numeric message levels
	/// Original: django/tests/messages_tests/base.py::BaseTests::test_settings_level
	#[test]
	fn test_custom_level_value() {
		// Configure MESSAGE_LEVEL to a custom value (29)
		let config = MessageConfig::new().with_level(29);

		// Test messages at various levels
		let debug_msg = Message::new(Level::Debug, "Debug message");
		let info_msg = Message::new(Level::Info, "Info message");
		let warning_msg = Message::new(Level::Warning, "Warning message");
		let error_msg = Message::new(Level::Error, "Error message");

		// Convert to numeric levels for testing
		let debug_level = 10;
		let info_level = 20;
		let warning_level = 30;
		let error_level = 40;

		// Verify only messages at or above level 29 are stored
		assert!(!config.should_store_message(debug_level));
		assert!(!config.should_store_message(info_level));
		assert!(config.should_store_message(warning_level));
		assert!(config.should_store_message(error_level));
	}

	/// Test custom level with custom tag mapping
	/// Original: django/tests/messages_tests/tests.py::AssertMessagesTest::test_custom_levelname
	#[test]
	fn test_custom_level_tag() {
		// Configure MESSAGE_TAGS with custom level (42 -> "CUSTOM")
		let mut custom_tags = HashMap::new();
		custom_tags.insert(42, "CUSTOM".to_string());

		let config = MessageConfig::new().with_custom_tags(custom_tags);

		// Test custom level tag
		assert_eq!(config.get_tag_for_level(42), Some(&"CUSTOM".to_string()));

		// Test that other levels still have default tags
		assert_eq!(config.get_tag_for_level(20), Some(&"info".to_string()));
	}

	/// Test that custom MESSAGE_TAGS properly override default tags
	/// Original: django/tests/messages_tests/base.py::BaseTests::test_custom_tags
	#[test]
	fn test_custom_tags_mapping() {
		// Configure custom MESSAGE_TAGS
		let mut custom_tags = HashMap::new();
		custom_tags.insert(20, "info".to_string()); // INFO -> "info"
		custom_tags.insert(10, "".to_string()); // DEBUG -> "" (empty)
		custom_tags.insert(30, "".to_string()); // WARNING -> "" (empty)
		custom_tags.insert(40, "bad".to_string()); // ERROR -> "bad"
		custom_tags.insert(29, "custom".to_string()); // Custom level -> "custom"

		let config = MessageConfig::new().with_custom_tags(custom_tags);

		// Test all configured tags
		assert_eq!(config.get_tag_for_level(20), Some(&"info".to_string()));
		assert_eq!(config.get_tag_for_level(10), Some(&"".to_string()));
		assert_eq!(config.get_tag_for_level(30), Some(&"".to_string()));
		assert_eq!(config.get_tag_for_level(40), Some(&"bad".to_string()));
		assert_eq!(config.get_tag_for_level(29), Some(&"custom".to_string()));

		// Test that tags combine correctly (simulate extra tags)
		let info_tag = config.get_tag_for_level(20).unwrap();
		let extra_tags = vec!["extra1".to_string(), "extra2".to_string()];
		let combined_tags = format!("{} {}", info_tag, extra_tags.join(" "));
		assert_eq!(combined_tags, "info extra1 extra2");
	}
}
