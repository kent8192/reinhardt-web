//! Messages framework integration tests
//!
//! Based on Django's messages tests from:
//! - django/tests/messages_tests/test_api.py

use reinhardt_contrib::{EnhancedMessage, MessageBuilder, MessageTag};
use reinhardt_messages::{Level as MessageLevel, Message};

#[test]
fn test_message_builder_info() {
    let msg = MessageBuilder::info("Information message").build();

    assert_eq!(msg.level, MessageLevel::Info);
    assert_eq!(msg.text, "Information message");
}

#[test]
fn test_message_builder_success() {
    let msg = MessageBuilder::success("Success message").build();

    assert_eq!(msg.level, MessageLevel::Success);
    assert_eq!(msg.text, "Success message");
}

#[test]
fn test_message_builder_warning() {
    let msg = MessageBuilder::warning("Warning message").build();

    assert_eq!(msg.level, MessageLevel::Warning);
    assert_eq!(msg.text, "Warning message");
}

#[test]
fn test_message_builder_error() {
    let msg = MessageBuilder::error("Error message").build();

    assert_eq!(msg.level, MessageLevel::Error);
    assert_eq!(msg.text, "Error message");
}

#[test]
fn test_message_builder_debug() {
    let msg = MessageBuilder::debug("Debug message").build();

    assert_eq!(msg.level, MessageLevel::Debug);
    assert_eq!(msg.text, "Debug message");
}

#[test]
fn test_message_builder_with_extra_tags() {
    let msg = MessageBuilder::info("Tagged message")
        .with_tag("dismissible")
        .with_tag("sticky")
        .build();

    assert_eq!(msg.extra_tags.len(), 2);
    assert!(msg.extra_tags.contains(&"dismissible".to_string()));
    assert!(msg.extra_tags.contains(&"sticky".to_string()));
}

#[test]
fn test_message_tag_css_classes() {
    assert_eq!(MessageTag::Debug.css_class(), "debug");
    assert_eq!(MessageTag::Info.css_class(), "info");
    assert_eq!(MessageTag::Success.css_class(), "success");
    assert_eq!(MessageTag::Warning.css_class(), "warning");
    assert_eq!(MessageTag::Error.css_class(), "error");
}

#[test]
fn test_message_tag_bootstrap_classes() {
    assert_eq!(MessageTag::Debug.bootstrap_class(), "alert-secondary");
    assert_eq!(MessageTag::Info.bootstrap_class(), "alert-info");
    assert_eq!(MessageTag::Success.bootstrap_class(), "alert-success");
    assert_eq!(MessageTag::Warning.bootstrap_class(), "alert-warning");
    assert_eq!(MessageTag::Error.bootstrap_class(), "alert-danger");
}

#[test]
fn test_message_tag_icons() {
    assert_eq!(MessageTag::Debug.icon(), "fa-bug");
    assert_eq!(MessageTag::Info.icon(), "fa-info-circle");
    assert_eq!(MessageTag::Success.icon(), "fa-check-circle");
    assert_eq!(MessageTag::Warning.icon(), "fa-exclamation-triangle");
    assert_eq!(MessageTag::Error.icon(), "fa-times-circle");
}

#[test]
fn test_enhanced_message_creation() {
    let base_msg = Message {
        level: MessageLevel::Info,
        text: "Test message".to_string(),
        extra_tags: Vec::new(),
    };

    let enhanced = EnhancedMessage::new(base_msg);

    assert_eq!(enhanced.tag, MessageTag::Info);
    assert_eq!(enhanced.message.text, "Test message");
    assert!(enhanced.timestamp.is_some());
}

#[test]
fn test_enhanced_message_with_extra_data() {
    let base_msg = Message {
        level: MessageLevel::Success,
        text: "Data message".to_string(),
        extra_tags: Vec::new(),
    };

    let extra_data = serde_json::json!({
        "user_id": 123,
        "action": "created"
    });

    let enhanced = EnhancedMessage::new(base_msg).with_extra(extra_data);

    assert!(enhanced.extra.is_some());
}

#[test]
fn test_enhanced_message_css_class() {
    let msg = Message {
        level: MessageLevel::Error,
        text: "Error".to_string(),
        extra_tags: Vec::new(),
    };

    let enhanced = EnhancedMessage::new(msg);
    assert_eq!(enhanced.css_class(), "error");
}

#[test]
fn test_enhanced_message_bootstrap_class() {
    let msg = Message {
        level: MessageLevel::Warning,
        text: "Warning".to_string(),
        extra_tags: Vec::new(),
    };

    let enhanced = EnhancedMessage::new(msg);
    assert_eq!(enhanced.bootstrap_class(), "alert-warning");
}

#[test]
fn test_message_level_conversion_to_tag() {
    let info_tag: MessageTag = MessageLevel::Info.into();
    assert_eq!(info_tag, MessageTag::Info);

    let success_tag: MessageTag = MessageLevel::Success.into();
    assert_eq!(success_tag, MessageTag::Success);

    let error_tag: MessageTag = MessageLevel::Error.into();
    assert_eq!(error_tag, MessageTag::Error);
}
