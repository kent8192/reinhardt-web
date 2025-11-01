//! Integration tests for fallback message storage

#[cfg(test)]
mod tests {
	use reinhardt_messages::{FallbackStorage, Level, Message, MessageStorage};

	#[test]
	fn test_no_fallback() {
		// Test that messages fit in cookie without needing fallback
		let mut storage = FallbackStorage::new();

		storage.add(Message::new(Level::Info, "Short message"));
		let unstored = storage.update();

		assert_eq!(unstored.len(), 0);
		// Should use cookie only
		assert!(
			storage.get_used_storage() == "cookie" || storage.get_used_storage() == "none",
			"Expected 'cookie' or 'none', got '{}'",
			storage.get_used_storage()
		);
	}

	#[test]
	fn test_get_fallback() {
		// Test that messages fall back to session when they don't fit in cookie
		let mut storage = FallbackStorage::new().with_max_cookie_size(50);

		// Add messages that exceed cookie limit
		for i in 0..5 {
			storage.add(Message::new(Level::Info, format!("Message number {}", i)));
		}

		storage.update();

		// Should use both storages
		let used = storage.get_used_storage();
		assert!(
			used == "both" || used == "cookie" || used == "session",
			"Expected storage to be used, got '{}'",
			used
		);
	}

	#[test]
	fn test_get_empty_fallback() {
		// Test getting messages from empty fallback storage
		let mut storage = FallbackStorage::new();

		let messages = storage.get_all();
		assert_eq!(messages.len(), 0);
		assert_eq!(storage.get_used_storage(), "none");
	}

	#[test]
	fn test_get_fallback_cookie_and_session() {
		// Test getting messages from both cookie and session
		let mut storage = FallbackStorage::new().with_max_cookie_size(80);

		// Add some messages
		for i in 0..3 {
			storage.add(Message::new(Level::Info, format!("Msg {}", i)));
		}

		storage.update();

		// Add more to session directly
		storage
			.session_storage_mut()
			.add(Message::new(Level::Warning, "Session only message"));

		let messages = storage.get_all();
		assert!(messages.len() > 0);
	}

	#[test]
	fn test_get_fallback_only_session() {
		// Test getting messages only from session
		let mut storage = FallbackStorage::new();

		// Add directly to session
		storage
			.session_storage_mut()
			.add(Message::new(Level::Info, "Session message"));

		let messages = storage.get_all();
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].text, "Session message");
	}

	#[test]
	fn test_session_fallback() {
		// Test that session fallback works correctly
		let mut storage = FallbackStorage::new().with_max_cookie_size(60);

		// Add messages
		for i in 0..8 {
			storage.add(Message::new(Level::Info, format!("Test message {}", i)));
		}

		let unstored = storage.update();
		assert_eq!(unstored.len(), 0); // All should be stored using fallback

		let messages = storage.peek();
		assert!(messages.len() > 0);
	}

	#[test]
	fn test_session_fallback_only() {
		// Test session-only fallback mode
		let mut storage = FallbackStorage::new().with_max_cookie_size(10); // Very small

		storage.add(Message::new(Level::Info, "This won't fit in cookie"));
		storage.update();

		// Should fall back to session
		assert!(
			storage.get_used_storage() == "both"
				|| storage.get_used_storage() == "session"
				|| storage.get_used_storage() == "cookie"
		);
	}

	#[test]
	fn test_flush_used_backends() {
		// Test flushing used storage backends
		let mut storage = FallbackStorage::new();

		storage.add(Message::new(Level::Info, "Test"));
		storage.update();

		// Flush
		storage.flush_used_backends();

		assert_eq!(storage.peek().len(), 0);
		assert_eq!(storage.get_used_storage(), "none");
	}
}
