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

	#[tokio::test]
	async fn test_message_persistence_across_requests() {
		// Test: Messages added in one request should appear in the next request
		use async_trait::async_trait;
		use bytes::Bytes;
		use hyper::{HeaderMap, Method, StatusCode, Version};
		use reinhardt_http::{Request, Response};
		use reinhardt_messages::middleware::{MessagesContainer, MessagesMiddleware};
		use reinhardt_messages::{Level, MemoryStorage, Message};
		use reinhardt_types::{Handler, Middleware};
		use std::sync::Arc;

		// Mock handler that adds a message
		struct AddMessageHandler;

		#[async_trait]
		impl Handler for AddMessageHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				// Add a message during request processing
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					container.add(Message::new(Level::Success, "Message from first request"));
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Mock handler that checks for messages
		struct CheckMessageHandler;

		#[async_trait]
		impl Handler for CheckMessageHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				// Verify message from previous request exists
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					let messages = container.get_messages();
					assert_eq!(messages.len(), 1);
					assert_eq!(messages[0].text, "Message from first request");
					assert_eq!(messages[0].level, Level::Success);
				} else {
					panic!("MessagesContainer not found in request extensions");
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Create middleware with shared storage
		let storage = MemoryStorage::new();
		let middleware = MessagesMiddleware::new(storage);

		// First request: Add a message
		let request1 = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler1 = Arc::new(AddMessageHandler) as Arc<dyn Handler>;
		let _ = middleware.process(request1, handler1).await.unwrap();

		// Second request: Verify message appears
		let request2 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler2 = Arc::new(CheckMessageHandler) as Arc<dyn Handler>;
		let _ = middleware.process(request2, handler2).await.unwrap();
	}

	#[tokio::test]
	async fn test_message_cleanup_after_display() {
		// Test: Messages should be cleared after being retrieved
		use async_trait::async_trait;
		use bytes::Bytes;
		use hyper::{HeaderMap, Method, StatusCode, Version};
		use reinhardt_http::{Request, Response};
		use reinhardt_messages::middleware::{MessagesContainer, MessagesMiddleware};
		use reinhardt_messages::{Level, MemoryStorage, Message};
		use reinhardt_types::{Handler, Middleware};
		use std::sync::Arc;

		// Handler that retrieves messages (simulating display)
		struct DisplayMessagesHandler;

		#[async_trait]
		impl Handler for DisplayMessagesHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				// Get messages (simulating template rendering)
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					let messages = container.get_messages();
					assert_eq!(messages.len(), 1);
					// Clear messages after display
					container.clear();
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Handler that checks messages are cleared
		struct CheckEmptyHandler;

		#[async_trait]
		impl Handler for CheckEmptyHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				// Verify no messages remain
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					let messages = container.get_messages();
					assert_eq!(
						messages.len(),
						0,
						"Messages should be cleared after display"
					);
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Create middleware with shared storage
		let mut storage = MemoryStorage::new();
		storage.add(Message::new(Level::Info, "Test message"));
		let middleware = MessagesMiddleware::new(storage);

		// First request: Display messages
		let request1 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler1 = Arc::new(DisplayMessagesHandler) as Arc<dyn Handler>;
		let _ = middleware.process(request1, handler1).await.unwrap();

		// Second request: Verify messages were cleared
		let request2 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler2 = Arc::new(CheckEmptyHandler) as Arc<dyn Handler>;
		let _ = middleware.process(request2, handler2).await.unwrap();
	}

	#[tokio::test]
	async fn test_message_accumulation_across_multiple_requests() {
		// Test: Multiple messages can be added and accumulated across requests
		use async_trait::async_trait;
		use bytes::Bytes;
		use hyper::{HeaderMap, Method, StatusCode, Version};
		use reinhardt_http::{Request, Response};
		use reinhardt_messages::middleware::{MessagesContainer, MessagesMiddleware};
		use reinhardt_messages::{Level, MemoryStorage, Message};
		use reinhardt_types::{Handler, Middleware};
		use std::sync::Arc;

		// Handler that adds a message
		struct AddMessageHandler {
			message_text: String,
			level: Level,
		}

		#[async_trait]
		impl Handler for AddMessageHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					container.add(Message::new(self.level, &self.message_text));
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Handler that checks accumulated messages
		struct CheckAccumulatedHandler {
			expected_count: usize,
		}

		#[async_trait]
		impl Handler for CheckAccumulatedHandler {
			async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
				if let Some(container) = request.extensions.get::<MessagesContainer>() {
					let messages = container.get_messages();
					assert_eq!(messages.len(), self.expected_count);
				}
				Ok(Response::new(StatusCode::OK))
			}
		}

		// Create middleware with shared storage
		let storage = MemoryStorage::new();
		let middleware = MessagesMiddleware::new(storage);

		// Request 1: Add first message
		let request1 = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler1 = Arc::new(AddMessageHandler {
			message_text: "First message".to_string(),
			level: Level::Info,
		}) as Arc<dyn Handler>;
		let _ = middleware.process(request1, handler1).await.unwrap();

		// Request 2: Add second message
		let request2 = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler2 = Arc::new(AddMessageHandler {
			message_text: "Second message".to_string(),
			level: Level::Warning,
		}) as Arc<dyn Handler>;
		let _ = middleware.process(request2, handler2).await.unwrap();

		// Request 3: Verify both messages accumulated
		let request3 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let handler3 = Arc::new(CheckAccumulatedHandler { expected_count: 2 }) as Arc<dyn Handler>;
		let _ = middleware.process(request3, handler3).await.unwrap();
	}
}
