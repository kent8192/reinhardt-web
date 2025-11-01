//! Context processor for template integration
//!
//! This module provides utilities for integrating messages with template engines.
//! Messages can be added to the template context for rendering in views.

use crate::message::Message;
use crate::middleware::MessagesContainer;
use reinhardt_http::Request;
use serde::Serialize;

/// Messages context for template rendering
///
/// This struct wraps messages for easy serialization into template contexts.
///
/// ## Example
///
/// ```rust
/// use reinhardt_messages::context::{MessagesContext, get_messages_context};
/// use reinhardt_messages::Message;
///
/// let messages = vec![Message::info("Hello"), Message::success("Done")];
/// let context = MessagesContext::new(messages);
/// assert_eq!(context.messages.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct MessagesContext {
	/// All messages to be displayed
	pub messages: Vec<Message>,
}

impl MessagesContext {
	/// Create a new messages context
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::context::MessagesContext;
	/// use reinhardt_messages::Message;
	///
	/// let messages = vec![Message::info("Test")];
	/// let context = MessagesContext::new(messages);
	/// assert_eq!(context.messages.len(), 1);
	/// ```
	pub fn new(messages: Vec<Message>) -> Self {
		Self { messages }
	}

	/// Create an empty messages context
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::context::MessagesContext;
	///
	/// let context = MessagesContext::empty();
	/// assert_eq!(context.messages.len(), 0);
	/// ```
	pub fn empty() -> Self {
		Self {
			messages: Vec::new(),
		}
	}

	/// Check if there are any messages
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::context::MessagesContext;
	/// use reinhardt_messages::Message;
	///
	/// let empty_context = MessagesContext::empty();
	/// assert!(!empty_context.has_messages());
	///
	/// let context = MessagesContext::new(vec![Message::info("Test")]);
	/// assert!(context.has_messages());
	/// ```
	pub fn has_messages(&self) -> bool {
		!self.messages.is_empty()
	}

	/// Get the number of messages
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_messages::context::MessagesContext;
	/// use reinhardt_messages::Message;
	///
	/// let context = MessagesContext::new(vec![
	///     Message::info("One"),
	///     Message::success("Two"),
	/// ]);
	/// assert_eq!(context.count(), 2);
	/// ```
	pub fn count(&self) -> usize {
		self.messages.len()
	}
}

/// Extract messages from request and create a context
///
/// This function is designed to be used as a context processor in template engines.
/// It retrieves messages from the request extensions and wraps them in a
/// MessagesContext for easy serialization.
///
/// # Example
///
/// ```rust
/// use reinhardt_messages::context::get_messages_context;
/// use reinhardt_messages::middleware::MessagesContainer;
/// use reinhardt_messages::Message;
/// use reinhardt_http::Request;
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
///
/// let mut request = Request::new(
///     Method::GET,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     HeaderMap::new(),
///     Bytes::new(),
/// );
///
/// let container = MessagesContainer::new(vec![Message::info("Test")]);
/// request.extensions.insert(container);
///
/// let context = get_messages_context(&request);
/// assert_eq!(context.messages.len(), 1);
/// ```
pub fn get_messages_context(request: &Request) -> MessagesContext {
	if let Some(container) = request.extensions.get::<MessagesContainer>() {
		MessagesContext::new(container.get_messages())
	} else {
		MessagesContext::empty()
	}
}

/// Add a message to the request's message container
///
/// This is a convenience function for adding messages to the request during
/// request processing.
///
/// # Example
///
/// ```rust
/// use reinhardt_messages::context::add_message;
/// use reinhardt_messages::middleware::MessagesContainer;
/// use reinhardt_messages::Message;
/// use reinhardt_http::Request;
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
///
/// let mut request = Request::new(
///     Method::GET,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     HeaderMap::new(),
///     Bytes::new(),
/// );
///
/// // Initialize container
/// request.extensions.insert(MessagesContainer::new(vec![]));
///
/// // Add message
/// add_message(&request, Message::success("Operation completed"));
///
/// // Verify message was added
/// if let Some(container) = request.extensions.get::<MessagesContainer>() {
///     assert_eq!(container.get_messages().len(), 1);
/// }
/// ```
pub fn add_message(request: &Request, message: Message) {
	if let Some(container) = request.extensions.get::<MessagesContainer>() {
		container.add(message);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Level;
	use crate::middleware::MessagesContainer;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};

	fn create_test_request() -> Request {
		Request::new(
			Method::GET,
			"/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		)
	}

	#[test]
	fn test_messages_context_new() {
		let messages = vec![Message::info("Test 1"), Message::success("Test 2")];
		let context = MessagesContext::new(messages);
		assert_eq!(context.messages.len(), 2);
		assert_eq!(context.messages[0].text, "Test 1");
		assert_eq!(context.messages[1].text, "Test 2");
	}

	#[test]
	fn test_messages_context_empty() {
		let context = MessagesContext::empty();
		assert_eq!(context.messages.len(), 0);
		assert!(!context.has_messages());
	}

	#[test]
	fn test_messages_context_has_messages() {
		let empty_context = MessagesContext::empty();
		assert!(!empty_context.has_messages());

		let context = MessagesContext::new(vec![Message::info("Test")]);
		assert!(context.has_messages());
	}

	#[test]
	fn test_messages_context_count() {
		let context = MessagesContext::new(vec![
			Message::info("One"),
			Message::success("Two"),
			Message::error("Three"),
		]);
		assert_eq!(context.count(), 3);
	}

	#[test]
	fn test_get_messages_context_with_messages() {
		let request = create_test_request();
		let container = MessagesContainer::new(vec![
			Message::info("Message 1"),
			Message::warning("Message 2"),
		]);
		request.extensions.insert(container);

		let context = get_messages_context(&request);
		assert_eq!(context.messages.len(), 2);
		assert_eq!(context.messages[0].text, "Message 1");
		assert_eq!(context.messages[1].text, "Message 2");
	}

	#[test]
	fn test_get_messages_context_without_container() {
		let request = create_test_request();
		let context = get_messages_context(&request);
		assert_eq!(context.messages.len(), 0);
		assert!(!context.has_messages());
	}

	#[test]
	fn test_add_message_to_request() {
		let request = create_test_request();
		request.extensions.insert(MessagesContainer::new(vec![]));

		add_message(&request, Message::success("Success message"));
		add_message(&request, Message::error("Error message"));

		if let Some(container) = request.extensions.get::<MessagesContainer>() {
			let messages = container.get_messages();
			assert_eq!(messages.len(), 2);
			assert_eq!(messages[0].text, "Success message");
			assert_eq!(messages[1].text, "Error message");
		} else {
			panic!("Container not found");
		}
	}

	#[test]
	fn test_add_message_without_container() {
		let request = create_test_request();
		// Should not panic, just do nothing
		add_message(&request, Message::info("Test"));
	}

	#[test]
	fn test_messages_context_serialization() {
		let messages = vec![
			Message::info("Info message"),
			Message::success("Success message"),
		];
		let context = MessagesContext::new(messages);

		// Test that it can be serialized
		let json = serde_json::to_string(&context).unwrap();
		assert!(json.contains("Info message"));
		assert!(json.contains("Success message"));
	}

	#[test]
	fn test_messages_context_with_all_levels() {
		let messages = vec![
			Message::debug("Debug"),
			Message::info("Info"),
			Message::success("Success"),
			Message::warning("Warning"),
			Message::error("Error"),
		];
		let context = MessagesContext::new(messages);

		assert_eq!(context.count(), 5);
		assert_eq!(context.messages[0].level, Level::Debug);
		assert_eq!(context.messages[1].level, Level::Info);
		assert_eq!(context.messages[2].level, Level::Success);
		assert_eq!(context.messages[3].level, Level::Warning);
		assert_eq!(context.messages[4].level, Level::Error);
	}
}
