//! Integration tests for view-related message functionality

#[cfg(test)]
mod tests {
	use reinhardt_integration_tests::message_middleware_mock::MockSuccessMessageMixin;
	use reinhardt_messages::{CookieStorage, Level, Message, MessageStorage, SessionStorage};
	use std::collections::HashMap;

	#[test]
	fn test_set_messages_success() {
		// Test SuccessMessageMixin adds success message after form submission
		let mixin = MockSuccessMessageMixin::new("Successfully saved {name}");

		let mut context = HashMap::new();
		context.insert("name".to_string(), "TestObject".to_string());

		// Simulate form submission
		let message = mixin.form_valid(&context);

		// Add to storage
		let mut storage = MemoryStorage::new();
		storage.add(message);

		// Verify success message is added to messages storage
		let messages = storage.peek();
		assert_eq!(messages.len(), 1);

		// Verify message contains expected text with interpolated data
		assert_eq!(messages[0].text, "Successfully saved TestObject");
		assert_eq!(messages[0].level, Level::Success);
	}

	#[test]
	fn test_set_messages_success_on_delete() {
		// Test SuccessMessageMixin works with DeleteView
		let mixin = MockSuccessMessageMixin::new("Successfully deleted {name}");

		let mut context = HashMap::new();
		context.insert("name".to_string(), "TestObject".to_string());

		// Create an object to delete (simulated)
		// Use DeleteView with SuccessMessageMixin
		let message = mixin.delete_success(&context);

		// Store in cookie storage (for redirect)
		let mut storage = CookieStorage::new();
		storage.add(message);

		let (cookie_value, unstored) = storage.get_cookie_value().unwrap();
		assert_eq!(unstored.len(), 0);

		// Follow redirect - load from cookie
		let mut new_storage = CookieStorage::new();
		new_storage.load_from_cookie(&cookie_value).unwrap();

		// Verify success message appears in response
		let messages = new_storage.peek();
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].text, "Successfully deleted TestObject");
		assert_eq!(messages[0].level, Level::Success);
	}

	#[test]
	fn test_with_template_response() {
		// Test that messages work with TemplateResponse
		let mut storage = SessionStorage::new();

		// Add messages at various levels
		storage.add(Message::new(Level::Debug, "Debug message"));
		storage.add(Message::new(Level::Info, "Info message"));
		storage.add(Message::new(Level::Warning, "Warning message"));

		// Render a template response (simulated)
		// Verify messages appear in template context
		let messages_for_template = storage.peek();
		assert_eq!(messages_for_template.len(), 3);
		assert_eq!(messages_for_template[0].text, "Debug message");
		assert_eq!(messages_for_template[1].text, "Info message");
		assert_eq!(messages_for_template[2].text, "Warning message");

		// Consume messages after first render
		let consumed = storage.get_all();
		assert_eq!(consumed.len(), 3);

		// Make a second GET request
		let messages_second_request = storage.peek();

		// Verify messages don't appear again (consumed after first render)
		assert_eq!(messages_second_request.len(), 0);
	}

	#[test]
	fn test_context_processor_message_levels() {
		// Test that message level constants are available in template context
		use reinhardt_messages::Level;

		// Render a template (simulated)
		// Verify DEFAULT_MESSAGE_LEVELS is in context
		let default_message_levels = HashMap::from([
			("DEBUG", Level::Debug.value()),
			("INFO", Level::Info.value()),
			("SUCCESS", Level::Success.value()),
			("WARNING", Level::Warning.value()),
			("ERROR", Level::Error.value()),
		]);

		// Verify it contains correct level constants
		assert_eq!(default_message_levels["DEBUG"], 10);
		assert_eq!(default_message_levels["INFO"], 20);
		assert_eq!(default_message_levels["SUCCESS"], 25);
		assert_eq!(default_message_levels["WARNING"], 30);
		assert_eq!(default_message_levels["ERROR"], 40);

		// Verify ordering
		assert!(default_message_levels["DEBUG"] < default_message_levels["INFO"]);
		assert!(default_message_levels["INFO"] < default_message_levels["SUCCESS"]);
		assert!(default_message_levels["SUCCESS"] < default_message_levels["WARNING"]);
		assert!(default_message_levels["WARNING"] < default_message_levels["ERROR"]);
	}

	use reinhardt_messages::MemoryStorage;
}
