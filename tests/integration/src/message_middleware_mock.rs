//! Mock MessageMiddleware for testing
//!
//! This provides a simplified middleware implementation for testing
//! message functionality without requiring the full HTTP stack.

use reinhardt_messages::{Level, Message, MessageStorage};
use std::collections::HashMap;

/// Mock MessageMiddleware for testing
pub struct MockMessageMiddleware {
    enabled: bool,
    fail_silently: bool,
}

impl MockMessageMiddleware {
    /// Create a new middleware with default settings (enabled)
    pub fn new() -> Self {
        Self {
            enabled: true,
            fail_silently: false,
        }
    }

    /// Set whether the middleware is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set whether to fail silently when middleware is disabled
    pub fn with_fail_silently(mut self, fail_silently: bool) -> Self {
        self.fail_silently = fail_silently;
        self
    }

    /// Add a message to storage
    ///
    /// Returns an error if middleware is disabled and fail_silently is false
    pub fn add_message(
        &self,
        storage: &mut dyn MessageStorage,
        level: Level,
        text: impl Into<String>,
    ) -> Result<(), String> {
        if !self.enabled {
            if self.fail_silently {
                // Silently ignore the message
                return Ok(());
            } else {
                return Err("MessageFailure: MessageMiddleware is not installed. \
                     Add MessageMiddleware to MIDDLEWARE in your settings."
                    .to_string());
            }
        }

        storage.add(Message::new(level, text));
        Ok(())
    }

    /// Process a request (simulated)
    pub fn process_request<R>(&self, _request: &R) -> Result<(), String> {
        // In real implementation, this would attach message storage to request
        Ok(())
    }

    /// Process a response (simulated)
    pub fn process_response<R>(&self, _response: &mut R) -> Result<(), String> {
        // In real implementation, this would handle message serialization
        // Should not fail even if messages don't exist on request
        Ok(())
    }
}

impl Default for MockMessageMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock SuccessMessageMixin for view testing
pub struct MockSuccessMessageMixin {
    pub success_message: String,
}

impl MockSuccessMessageMixin {
    /// Create a new SuccessMessageMixin with a success message template
    pub fn new(success_message: impl Into<String>) -> Self {
        Self {
            success_message: success_message.into(),
        }
    }

    /// Get the success message with context interpolation
    pub fn get_success_message(&self, context: &HashMap<String, String>) -> String {
        let mut msg = self.success_message.clone();
        for (key, value) in context {
            msg = msg.replace(&format!("{{{}}}", key), value);
        }
        msg
    }

    /// Simulate form submission success
    pub fn form_valid(&self, context: &HashMap<String, String>) -> Message {
        let message_text = self.get_success_message(context);
        Message::new(Level::Success, message_text)
    }

    /// Simulate delete success
    pub fn delete_success(&self, context: &HashMap<String, String>) -> Message {
        let message_text = self.get_success_message(context);
        Message::new(Level::Success, message_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_messages::MemoryStorage;

    #[test]
    fn test_middleware_enabled() {
        let middleware = MockMessageMiddleware::new();
        let mut storage = MemoryStorage::new();

        let result = middleware.add_message(&mut storage, Level::Info, "Test");
        assert!(result.is_ok());
        assert_eq!(storage.peek().len(), 1);
    }

    #[test]
    fn test_middleware_disabled_error() {
        let middleware = MockMessageMiddleware::new().with_enabled(false);
        let mut storage = MemoryStorage::new();

        let result = middleware.add_message(&mut storage, Level::Info, "Test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("MessageMiddleware is not installed")
        );
    }

    #[test]
    fn test_middleware_disabled_silent() {
        let middleware = MockMessageMiddleware::new()
            .with_enabled(false)
            .with_fail_silently(true);
        let mut storage = MemoryStorage::new();

        let result = middleware.add_message(&mut storage, Level::Info, "Test");
        assert!(result.is_ok());
        assert_eq!(storage.peek().len(), 0); // Message not added
    }

    #[test]
    fn test_success_message_mixin() {
        let mixin = MockSuccessMessageMixin::new("Successfully saved {name}");
        let mut context = HashMap::new();
        context.insert("name".to_string(), "TestObject".to_string());

        let message = mixin.form_valid(&context);
        assert_eq!(message.text, "Successfully saved TestObject");
        assert_eq!(message.level, Level::Success);
    }
}
