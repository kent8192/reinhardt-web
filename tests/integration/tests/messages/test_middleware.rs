//! Integration tests for message middleware

#[cfg(test)]
mod tests {
	use reinhardt_integration_tests::message_middleware_mock::MockMessageMiddleware;
	use reinhardt_integration_tests::messages_helpers::MockResponse;
	use reinhardt_messages::{
		CookieStorage, Level, MemoryStorage, Message, MessageStorage, SessionStorage,
	};

	#[test]
	fn test_response_without_messages() {
		// Test that MessageMiddleware tolerates messages not existing on request
		let middleware = MockMessageMiddleware::new();
		let mut response = MockResponse::new();

		// Should not panic even though messages don't exist on request
		let result = middleware.process_response(&mut response);
		assert!(result.is_ok());
	}

	#[test]
	fn test_full_request_response_cycle() {
		// Test that messages are properly stored and retrieved across full request/response cycle
		let mut storage = CookieStorage::new();

		// 1. Create a request with message middleware enabled and add messages at different levels
		storage.add(Message::new(Level::Debug, "Debug msg"));
		storage.add(Message::new(Level::Info, "Info msg"));
		storage.add(Message::new(Level::Success, "Success msg"));
		storage.add(Message::new(Level::Warning, "Warning msg"));
		storage.add(Message::new(Level::Error, "Error msg"));

		// 2. Perform a POST request (simulated by updating storage)
		let unstored = storage.update();
		assert_eq!(unstored.len(), 0, "All messages should fit");

		// 3. Get cookie value for transmission
		let (cookie_value, _) = storage.get_cookie_value().unwrap();

		// 4. Follow redirect - simulate GET request with cookie
		let mut new_storage = CookieStorage::new();
		new_storage.load_from_cookie(&cookie_value).unwrap();

		// 5. Verify messages appear in the response context
		let messages = new_storage.peek();
		assert_eq!(messages.len(), 5);
		assert_eq!(messages[0].level, Level::Debug);
		assert_eq!(messages[1].level, Level::Info);
		assert_eq!(messages[2].level, Level::Success);
		assert_eq!(messages[3].level, Level::Warning);
		assert_eq!(messages[4].level, Level::Error);

		// 6. Verify messages are cleared after being consumed
		let consumed = new_storage.get_all();
		assert_eq!(consumed.len(), 5);
		assert_eq!(new_storage.peek().len(), 0);
	}

	#[test]
	fn test_multiple_posts() {
		// Test that messages persist properly when multiple POSTs are made before a GET
		let mut storage = SessionStorage::new();

		// 1. Make multiple POST requests, each adding messages
		storage.add(Message::new(Level::Info, "Post 1 message 1"));
		storage.add(Message::new(Level::Info, "Post 1 message 2"));

		// Simulate second POST
		storage.add(Message::new(Level::Warning, "Post 2 message 1"));
		storage.add(Message::new(Level::Warning, "Post 2 message 2"));

		// Simulate third POST
		storage.add(Message::new(Level::Success, "Post 3 message 1"));

		// 2. Messages should accumulate across POST requests
		let messages = storage.peek();
		assert_eq!(messages.len(), 5);

		// 3. Make a GET request
		// 4. Verify all messages from all POSTs are present
		assert_eq!(messages[0].text, "Post 1 message 1");
		assert_eq!(messages[1].text, "Post 1 message 2");
		assert_eq!(messages[2].text, "Post 2 message 1");
		assert_eq!(messages[3].text, "Post 2 message 2");
		assert_eq!(messages[4].text, "Post 3 message 1");

		// 5. Verify messages are cleared after being consumed
		let consumed = storage.get_all();
		assert_eq!(consumed.len(), 5);
		assert_eq!(storage.peek().len(), 0);
	}

	#[test]
	fn test_middleware_disabled() {
		// Test that an exception is raised when middleware is disabled and messages are added
		let middleware = MockMessageMiddleware::new().with_enabled(false);
		let mut storage = MemoryStorage::new();

		// 1. Configure the system with message middleware disabled
		// 2. Attempt to add a message
		let result = middleware.add_message(&mut storage, Level::Info, "Test message");

		// 3. Verify that a MessageFailure error is raised
		assert!(result.is_err());
		let error = result.unwrap_err();
		assert!(
			error.contains("MessageFailure")
				|| error.contains("MessageMiddleware is not installed"),
			"Expected MessageFailure error, got: {}",
			error
		);

		// Verify no message was added
		assert_eq!(storage.peek().len(), 0);
	}

	#[test]
	fn test_middleware_disabled_fail_silently() {
		// Test that no exception is raised when middleware is disabled and fail_silently=True
		let middleware = MockMessageMiddleware::new()
			.with_enabled(false)
			.with_fail_silently(true);
		let mut storage = MemoryStorage::new();

		// 1. Configure the system with message middleware disabled
		// 2. Attempt to add a message with fail_silently=true
		let result = middleware.add_message(&mut storage, Level::Info, "Test message");

		// 3. Verify that no error is raised
		assert!(result.is_ok());

		// 4. Verify that no messages are stored
		assert_eq!(storage.peek().len(), 0);
	}
}
