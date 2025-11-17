//! Integration tests for message API
//!
//! These tests verify the basic message API functionality using reinhardt-messages.

#[cfg(test)]
mod tests {
	use reinhardt_messages::{Level, MemoryStorage, Message, MessageStorage};

	// Basic API tests that can run without full HTTP integration

	#[test]
	fn test_message_creation_with_storage() {
		// Test intent: Verify Message creation and storage via MemoryStorage::add()
		// with correct text and level preservation
		// Not intent: Multiple messages, message ordering, complex levels
		let mut storage = MemoryStorage::new();

		// Create and add a message
		let message = Message::new(Level::Debug, "some message");
		storage.add(message);

		// Verify it was stored
		let stored = storage.peek();
		assert_eq!(stored.len(), 1);
		assert_eq!(stored[0].text, "some message");
		assert_eq!(stored[0].level, Level::Debug);
	}

	#[test]
	fn test_multiple_messages_ordering() {
		// Test intent: Verify MemoryStorage preserves insertion order
		// of multiple messages with different levels
		// Not intent: Message priority sorting, level-based filtering, message deduplication
		let mut storage = MemoryStorage::new();

		storage.add(Message::new(Level::Info, "First message"));
		storage.add(Message::new(Level::Warning, "Second message"));
		storage.add(Message::new(Level::Error, "Third message"));

		let messages = storage.peek();
		assert_eq!(messages.len(), 3);
		assert_eq!(messages[0].text, "First message");
		assert_eq!(messages[1].text, "Second message");
		assert_eq!(messages[2].text, "Third message");
	}

	#[test]
	fn test_message_with_extra_tags_api() {
		// Test intent: Verify Message::with_tags() correctly attaches
		// extra tags to message and persists in storage
		// Not intent: Tag validation, tag ordering, tag deduplication, max tag count
		let mut storage = MemoryStorage::new();

		let message = Message::new(Level::Info, "Tagged message")
			.with_tags(vec!["important".to_string(), "user-action".to_string()]);

		storage.add(message);

		let stored = storage.peek();
		assert_eq!(stored.len(), 1);
		assert_eq!(stored[0].extra_tags.len(), 2);
		assert!(stored[0].extra_tags.contains(&"important".to_string()));
		assert!(stored[0].extra_tags.contains(&"user-action".to_string()));
	}

	// HTTP/Middleware Integration Tests

	#[test]
	fn test_request_with_middleware() {
		// Test intent: Verify mock HTTP request can add and retrieve messages
		// via Arc<Mutex<MemoryStorage>> shared state pattern
		// Not intent: Real HTTP middleware, request lifecycle, concurrent access safety
		use std::collections::HashMap;
		use std::sync::{Arc, Mutex};

		// Mock HTTP request with message storage
		#[derive(Debug)]
		struct MockRequest {
			_headers: HashMap<String, String>,
			message_storage: Arc<Mutex<MemoryStorage>>,
		}

		impl MockRequest {
			fn new() -> Self {
				Self {
					_headers: HashMap::new(),
					message_storage: Arc::new(Mutex::new(MemoryStorage::new())),
				}
			}

			fn add_message(&self, message: Message) {
				self.message_storage.lock().unwrap().add(message);
			}

			fn get_messages(&self) -> Vec<Message> {
				self.message_storage.lock().unwrap().peek()
			}
		}

		let request = MockRequest::new();

		// Add messages to the request
		request.add_message(Message::new(Level::Info, "Request processed successfully"));
		request.add_message(Message::new(Level::Warning, "Deprecated API endpoint used"));

		// Verify messages are accessible via the request
		let messages = request.get_messages();
		assert_eq!(messages.len(), 2);
		assert_eq!(messages[0].text, "Request processed successfully");
		assert_eq!(messages[0].level, Level::Info);
		assert_eq!(messages[1].text, "Deprecated API endpoint used");
		assert_eq!(messages[1].level, Level::Warning);
	}

	#[test]
	fn test_request_is_none() {
		// Test intent: Verify RequestWrapper correctly returns error when
		// request is None and successfully stores messages when request is Some
		// Not intent: Error message format, logging behavior, fallback storage mechanisms
		use std::collections::HashMap;
		use std::sync::{Arc, Mutex};

		// Mock request wrapper that can be None
		#[derive(Debug)]
		struct RequestWrapper {
			request: Option<MockRequest>,
			message_storage: Arc<Mutex<MemoryStorage>>,
		}

		#[derive(Debug)]
		struct MockRequest {
			_headers: HashMap<String, String>,
		}

		impl RequestWrapper {
			fn new_with_request() -> Self {
				Self {
					request: Some(MockRequest {
						_headers: HashMap::new(),
					}),
					message_storage: Arc::new(Mutex::new(MemoryStorage::new())),
				}
			}

			fn new_without_request() -> Self {
				Self {
					request: None,
					message_storage: Arc::new(Mutex::new(MemoryStorage::new())),
				}
			}

			fn add_message(&self, message: Message) -> Result<(), String> {
				match &self.request {
					Some(_) => {
						// Add message to the actual storage
						let mut storage = self.message_storage.lock().unwrap();
						storage.add(message);
						Ok(())
					}
					None => Err("No request available".to_string()),
				}
			}

			fn get_messages(&self) -> Vec<Message> {
				let storage = self.message_storage.lock().unwrap();
				storage.peek()
			}
		}

		// Test with request present
		let wrapper_with_request = RequestWrapper::new_with_request();
		let result = wrapper_with_request.add_message(Message::new(Level::Info, "Test message"));
		assert!(result.is_ok());

		// Verify the message was actually stored
		let messages = wrapper_with_request.get_messages();
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].text, "Test message");
		assert_eq!(messages[0].level, Level::Info);

		// Add another message and verify both are stored
		let result2 =
			wrapper_with_request.add_message(Message::new(Level::Warning, "Second message"));
		assert!(result2.is_ok());

		let all_messages = wrapper_with_request.get_messages();
		assert_eq!(all_messages.len(), 2);
		assert_eq!(all_messages[1].text, "Second message");
		assert_eq!(all_messages[1].level, Level::Warning);

		// Test with no request
		let wrapper_without_request = RequestWrapper::new_without_request();
		let result = wrapper_without_request.add_message(Message::new(Level::Info, "Test message"));
		assert!(result.is_err());
		assert_eq!(result.unwrap_err(), "No request available");

		// Verify no messages were stored when request is None
		let no_messages = wrapper_without_request.get_messages();
		assert_eq!(no_messages.len(), 0);
	}

	#[test]
	fn test_middleware_missing() {
		// Test intent: Verify MockRequest::add_message() returns error
		// when has_message_middleware flag is false and succeeds when true
		// Not intent: Actual middleware installation, middleware chain execution, error types
		use std::collections::HashMap;

		#[derive(Debug)]
		struct MockRequest {
			_headers: HashMap<String, String>,
			has_message_middleware: bool,
		}

		impl MockRequest {
			fn new() -> Self {
				Self {
					_headers: HashMap::new(),
					has_message_middleware: false,
				}
			}

			fn with_middleware() -> Self {
				Self {
					_headers: HashMap::new(),
					has_message_middleware: true,
				}
			}

			fn add_message(&self, _message: Message) -> Result<(), String> {
				if self.has_message_middleware {
					Ok(())
				} else {
					Err("MessageMiddleware not installed".to_string())
				}
			}
		}

		// Test without middleware
		let request = MockRequest::new();
		let result = request.add_message(Message::new(Level::Debug, "some message"));
		assert!(result.is_err());
		assert_eq!(result.unwrap_err(), "MessageMiddleware not installed");

		// Test with middleware
		let request_with_middleware = MockRequest::with_middleware();
		let result =
			request_with_middleware.add_message(Message::new(Level::Debug, "some message"));
		assert!(result.is_ok());
	}

	#[test]
	fn test_middleware_missing_silently() {
		// Test intent: Verify add_message_silently() respects fail_silently parameter:
		// returns error when false, succeeds when true (even without middleware)
		// Not intent: Error suppression logging, partial failure handling, error recovery
		use std::collections::HashMap;

		#[derive(Debug)]
		struct MockRequest {
			_headers: HashMap<String, String>,
			has_message_middleware: bool,
		}

		impl MockRequest {
			fn new() -> Self {
				Self {
					_headers: HashMap::new(),
					has_message_middleware: false,
				}
			}

			fn add_message_silently(
				&self,
				_message: Message,
				fail_silently: bool,
			) -> Result<(), String> {
				if self.has_message_middleware {
					Ok(())
				} else if fail_silently {
					// Silently ignore the error
					Ok(())
				} else {
					Err("MessageMiddleware not installed".to_string())
				}
			}
		}

		let request = MockRequest::new();

		// Test with fail_silently=false (should fail)
		let result =
			request.add_message_silently(Message::new(Level::Debug, "some message"), false);
		assert!(result.is_err());

		// Test with fail_silently=true (should succeed silently)
		let result = request.add_message_silently(Message::new(Level::Debug, "some message"), true);
		assert!(result.is_ok());
	}

	#[test]
	fn test_custom_request_wrapper() {
		// Test intent: Verify CustomRequestWrapper can implement message storage
		// interface with additional request properties (path field)
		// Not intent: Trait-based polymorphism, request wrapper validation, type safety
		use std::collections::HashMap;
		use std::sync::{Arc, Mutex};

		// Custom request wrapper that implements the message storage interface
		#[derive(Debug)]
		struct CustomRequestWrapper {
			inner_request: MockRequest,
			message_storage: Arc<Mutex<MemoryStorage>>,
		}

		#[derive(Debug)]
		struct MockRequest {
			_headers: HashMap<String, String>,
			path: String,
		}

		impl CustomRequestWrapper {
			fn new(path: String) -> Self {
				Self {
					inner_request: MockRequest {
						_headers: HashMap::new(),
						path,
					},
					message_storage: Arc::new(Mutex::new(MemoryStorage::new())),
				}
			}

			fn add_message(&self, message: Message) {
				self.message_storage.lock().unwrap().add(message);
			}

			fn get_messages(&self) -> Vec<Message> {
				self.message_storage.lock().unwrap().peek()
			}

			fn get_path(&self) -> &str {
				&self.inner_request.path
			}
		}

		// Test with custom request wrapper
		let wrapper = CustomRequestWrapper::new("/api/users/".to_string());

		// Add messages through the wrapper
		wrapper.add_message(Message::new(Level::Info, "User list requested"));
		wrapper.add_message(Message::new(Level::Warning, "Large result set"));

		// Verify messages are stored and accessible
		let messages = wrapper.get_messages();
		assert_eq!(messages.len(), 2);
		assert_eq!(messages[0].text, "User list requested");
		assert_eq!(messages[1].text, "Large result set");

		// Verify other request properties work
		assert_eq!(wrapper.get_path(), "/api/users/");
	}
}
