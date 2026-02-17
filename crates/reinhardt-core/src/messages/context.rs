//! Context for template integration
//!
//! This module provides utilities for integrating messages with template engines.
//! Messages can be serialized and passed to templates for rendering.
//!
//! ## Note
//!
//! HTTP request integration functions (`get_messages_context`, `add_message`)
//! have been moved to `reinhardt-http` crate to prevent circular dependencies.

use super::message::Message;
use serde::Serialize;

/// Messages context for template rendering
///
/// This struct wraps messages for easy serialization into template contexts.
///
/// ## Example
///
/// ```rust
/// use reinhardt_core::messages::context::MessagesContext;
/// use reinhardt_core::messages::Message;
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
	/// use reinhardt_core::messages::context::MessagesContext;
	/// use reinhardt_core::messages::Message;
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
	/// use reinhardt_core::messages::context::MessagesContext;
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
	/// use reinhardt_core::messages::context::MessagesContext;
	/// use reinhardt_core::messages::Message;
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
	/// use reinhardt_core::messages::context::MessagesContext;
	/// use reinhardt_core::messages::Message;
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::messages::Level;
	use rstest::rstest;

	#[rstest]
	fn test_messages_context_new() {
		let messages = vec![Message::info("Test 1"), Message::success("Test 2")];
		let context = MessagesContext::new(messages);
		assert_eq!(context.messages.len(), 2);
		assert_eq!(context.messages[0].text, "Test 1");
		assert_eq!(context.messages[1].text, "Test 2");
	}

	#[rstest]
	fn test_messages_context_empty() {
		let context = MessagesContext::empty();
		assert_eq!(context.messages.len(), 0);
		assert!(!context.has_messages());
	}

	#[rstest]
	fn test_messages_context_has_messages() {
		let empty_context = MessagesContext::empty();
		assert!(!empty_context.has_messages());

		let context = MessagesContext::new(vec![Message::info("Test")]);
		assert!(context.has_messages());
	}

	#[rstest]
	fn test_messages_context_count() {
		let context = MessagesContext::new(vec![
			Message::info("One"),
			Message::success("Two"),
			Message::error("Three"),
		]);
		assert_eq!(context.count(), 3);
	}

	#[rstest]
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

	#[rstest]
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
