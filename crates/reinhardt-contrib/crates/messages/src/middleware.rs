//! Messages middleware for HTTP requests
//!
//! This module provides middleware that automatically loads and saves messages
//! for each HTTP request/response cycle.
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_messages::middleware::MessagesMiddleware;
//! use reinhardt_messages::storage::SessionStorage;
//!
//! // Create message storage backend
//! let storage = SessionStorage::new();
//!
//! // Create middleware
//! let middleware = MessagesMiddleware::new(storage);
//! ```

use crate::message::Message;
use crate::storage::MessageStorage;
use async_trait::async_trait;
use reinhardt_exception::Result;
use reinhardt_types::{Handler, Middleware};
use reinhardt_http::{Request, Response};
use std::sync::{Arc, Mutex};

/// Messages middleware
///
/// Automatically loads messages from storage on request and saves them on response.
///
/// ## Example
///
/// ```rust
/// use reinhardt_messages::middleware::MessagesMiddleware;
/// use reinhardt_messages::storage::MemoryStorage;
///
/// let storage = MemoryStorage::new();
/// let middleware = MessagesMiddleware::new(storage);
/// ```
pub struct MessagesMiddleware<S: MessageStorage + 'static> {
	storage: Arc<Mutex<S>>,
}

impl<S: MessageStorage + 'static> MessagesMiddleware<S> {
	/// Create a new messages middleware
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesMiddleware;
	/// use reinhardt_messages::storage::MemoryStorage;
	///
	/// let storage = MemoryStorage::new();
	/// let middleware = MessagesMiddleware::new(storage);
	/// ```
	pub fn new(storage: S) -> Self {
		Self {
			storage: Arc::new(Mutex::new(storage)),
		}
	}

	/// Get reference to storage backend
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesMiddleware;
	/// use reinhardt_messages::storage::MemoryStorage;
	///
	/// let storage = MemoryStorage::new();
	/// let middleware = MessagesMiddleware::new(storage);
	/// let storage_ref = middleware.storage();
	/// ```
	pub fn storage(&self) -> Arc<Mutex<S>> {
		self.storage.clone()
	}
}

#[async_trait]
impl<S: MessageStorage + 'static> Middleware for MessagesMiddleware<S> {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		// Load messages from storage and attach to request extensions
		let messages = {
			let mut storage = self.storage.lock().unwrap();
			storage.get_all()
		};

		// Store messages in request extensions for access during request processing
		let container = MessagesContainer::new(messages);
		request.extensions.insert(container.clone());

		// Process the request
		let response = next.handle(request).await?;

		// Save messages back to storage after request processing
		{
			// Extract messages from container (we have a clone)
			let messages_to_save = container.get_messages();

			// Persist messages to storage
			let mut storage = self.storage.lock().unwrap();
			// Clear old messages first
			storage.clear();
			// Add new messages
			for message in messages_to_save {
				storage.add(message);
			}
		}

		Ok(response)
	}
}

/// Container for messages stored in request extensions
///
/// This struct is used to store messages in the request extensions
/// during request processing.
///
/// ## Example
///
/// ```rust
/// use reinhardt_messages::middleware::MessagesContainer;
/// use reinhardt_messages::Message;
///
/// let container = MessagesContainer::new(vec![]);
/// container.add(Message::info("Hello"));
/// ```
#[derive(Clone)]
pub struct MessagesContainer {
	messages: Arc<Mutex<Vec<Message>>>,
}

impl MessagesContainer {
	/// Create a new messages container
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesContainer;
	/// use reinhardt_messages::Message;
	///
	/// let messages = vec![Message::info("Initial message")];
	/// let container = MessagesContainer::new(messages);
	/// ```
	pub fn new(messages: Vec<Message>) -> Self {
		Self {
			messages: Arc::new(Mutex::new(messages)),
		}
	}

	/// Add a message to the container
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesContainer;
	/// use reinhardt_messages::Message;
	///
	/// let container = MessagesContainer::new(vec![]);
	/// container.add(Message::success("Operation completed"));
	/// ```
	pub fn add(&self, message: Message) {
		let mut messages = self.messages.lock().unwrap();
		messages.push(message);
	}

	/// Get all messages from the container
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesContainer;
	/// use reinhardt_messages::Message;
	///
	/// let container = MessagesContainer::new(vec![Message::info("Test")]);
	/// let messages = container.get_messages();
	/// assert_eq!(messages.len(), 1);
	/// ```
	pub fn get_messages(&self) -> Vec<Message> {
		let messages = self.messages.lock().unwrap();
		messages.clone()
	}

	/// Clear all messages from the container
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::middleware::MessagesContainer;
	/// use reinhardt_messages::Message;
	///
	/// let container = MessagesContainer::new(vec![Message::info("Test")]);
	/// container.clear();
	/// assert_eq!(container.get_messages().len(), 0);
	/// ```
	pub fn clear(&self) {
		let mut messages = self.messages.lock().unwrap();
		messages.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Level;
	use crate::storage::MemoryStorage;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Uri, Version};

	// Mock handler for testing
	struct MockHandler;

	#[async_trait]
	impl Handler for MockHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::new(StatusCode::OK))
		}
	}

	fn create_test_request() -> Request {
		Request::new(
			Method::GET,
			"/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		)
	}

	#[tokio::test]
	async fn test_messages_middleware_new() {
		let storage = MemoryStorage::new();
		let _middleware = MessagesMiddleware::new(storage);
	}

	#[tokio::test]
	async fn test_messages_middleware_storage() {
		let storage = MemoryStorage::new();
		let middleware = MessagesMiddleware::new(storage);
		let _storage_ref = middleware.storage();
	}

	#[tokio::test]
	async fn test_middleware_loads_existing_messages() {
		let mut storage = MemoryStorage::new();
		storage.add(Message::info("Pre-existing message"));

		let middleware = MessagesMiddleware::new(storage);
		let handler = Arc::new(MockHandler);
		let request = create_test_request();

		let _response = middleware.process(request, handler).await.unwrap();

		// Verify that storage still contains the message (persisted after request)
		let storage_ref = middleware.storage();
		let storage = storage_ref.lock().unwrap();
		assert_eq!(storage.peek().len(), 1);
		assert_eq!(storage.peek()[0].text, "Pre-existing message");
	}

	#[tokio::test]
	async fn test_middleware_provides_container() {
		let storage = MemoryStorage::new();
		let middleware = MessagesMiddleware::new(storage);

		// Handler that checks for container
		struct ContainerCheckHandler;

		#[async_trait]
		impl Handler for ContainerCheckHandler {
			async fn handle(&self, request: Request) -> Result<Response> {
				// Verify container exists
				assert!(request.extensions.get::<MessagesContainer>().is_some());
				Ok(Response::new(StatusCode::OK))
			}
		}

		let handler = Arc::new(ContainerCheckHandler);
		let request = create_test_request();

		let _response = middleware.process(request, handler).await.unwrap();
	}

	#[tokio::test]
	async fn test_messages_container_new() {
		let messages = vec![Message::info("Test message")];
		let container = MessagesContainer::new(messages);
		let loaded_messages = container.get_messages();
		assert_eq!(loaded_messages.len(), 1);
		assert_eq!(loaded_messages[0].text, "Test message");
	}

	#[tokio::test]
	async fn test_messages_container_add() {
		let container = MessagesContainer::new(vec![]);
		container.add(Message::success("Success"));
		container.add(Message::error("Error"));

		let messages = container.get_messages();
		assert_eq!(messages.len(), 2);
		assert_eq!(messages[0].text, "Success");
		assert_eq!(messages[1].text, "Error");
	}

	#[tokio::test]
	async fn test_messages_container_clear() {
		let container = MessagesContainer::new(vec![Message::info("Test")]);
		assert_eq!(container.get_messages().len(), 1);

		container.clear();
		assert_eq!(container.get_messages().len(), 0);
	}

	#[tokio::test]
	async fn test_messages_container_with_different_levels() {
		let container = MessagesContainer::new(vec![]);
		container.add(Message::debug("Debug message"));
		container.add(Message::info("Info message"));
		container.add(Message::success("Success message"));
		container.add(Message::warning("Warning message"));
		container.add(Message::error("Error message"));

		let messages = container.get_messages();
		assert_eq!(messages.len(), 5);
		assert_eq!(messages[0].level, Level::Debug);
		assert_eq!(messages[1].level, Level::Info);
		assert_eq!(messages[2].level, Level::Success);
		assert_eq!(messages[3].level, Level::Warning);
		assert_eq!(messages[4].level, Level::Error);
	}
}
