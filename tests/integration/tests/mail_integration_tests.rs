//! Email Integration Tests
//!
//! Integration tests for reinhardt-mail working with reinhardt-signals.
//! These tests verify that email functionality integrates properly with
//! signal handling.

use reinhardt_mail::{EmailBackend, EmailMessage, MemoryBackend};
use reinhardt_signals::Signal;
use std::sync::{Arc, Mutex};

// ========== Basic Mail Backend Tests ==========

#[tokio::test]
async fn test_send_simple_email() {
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Test Email")
        .body("This is a test email body")
        .from("noreply@example.com")
        .to(vec!["alice@example.com"])
        .build()
        .unwrap();

    backend.send(&message).await.unwrap();

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].subject, "Test Email");
    assert!(messages[0].body.contains("This is a test email body"));
}

#[tokio::test]
async fn test_send_email_with_html() {
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Order Confirmation")
        .body("Order #12345 confirmed\n\nTotal: $99.99")
        .html("<html><body><h1>Order #12345 confirmed</h1><p>Total: $99.99</p></body></html>")
        .from("orders@example.com")
        .to(vec!["customer@example.com"])
        .build()
        .unwrap();

    backend.send(&message).await.unwrap();

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].body.contains("Order #12345 confirmed"));

    let html = messages[0].html_body.as_ref().unwrap();
    assert!(html.contains("<h1>Order #12345 confirmed</h1>"));
}

// ========== Signals Integration Tests ==========

/// Create a new pre-send signal for each test
fn create_pre_send_signal() -> Signal<EmailMessage> {
    Signal::new("email_pre_send")
}

/// Create a new post-send signal for each test
fn create_post_send_signal() -> Signal<(EmailMessage, bool)> {
    Signal::new("email_post_send")
}

#[tokio::test]
async fn test_pre_send_signal_fired() {
    let email_pre_send = create_pre_send_signal();

    // Track if signal was called
    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();

    // Connect receiver to pre-send signal
    email_pre_send.connect(move |message: Arc<EmailMessage>| {
        let called = called_clone.clone();
        async move {
            *called.lock().unwrap() = true;

            // Verify message data is accessible
            assert_eq!(message.subject, "Test Signal");
            assert_eq!(message.from_email, "sender@example.com");

            Ok(())
        }
    });

    // Create and send email
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Test Signal")
        .body("Testing pre-send signal")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    // Fire pre-send signal
    email_pre_send.send(message.clone()).await.unwrap();

    // Send actual email
    backend.send(&message).await.unwrap();

    // Verify signal was called
    assert!(*called.lock().unwrap());

    // Verify email was sent
    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_post_send_signal_with_result() {
    let email_post_send = create_post_send_signal();

    // Track if signal was called and what result was
    let signal_data = Arc::new(Mutex::new(None));
    let signal_data_clone = signal_data.clone();

    // Connect receiver to post-send signal
    email_post_send.connect(move |data: Arc<(EmailMessage, bool)>| {
        let signal_data = signal_data_clone.clone();
        async move {
            let (message, success) = &*data;

            // Store signal data
            *signal_data.lock().unwrap() =
                Some((message.subject.clone(), message.to.clone(), *success));

            Ok(())
        }
    });

    // Create and send email
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Post Send Test")
        .body("Testing post-send signal")
        .from("sender@example.com")
        .to(vec!["recipient1@example.com", "recipient2@example.com"])
        .build()
        .unwrap();

    // Send email
    let send_result = backend.send(&message).await;
    let success = send_result.is_ok();

    // Fire post-send signal with result
    email_post_send
        .send((message.clone(), success))
        .await
        .unwrap();

    // Verify signal was called with correct data
    let data = signal_data.lock().unwrap();
    assert!(data.is_some());

    let (subject, recipients, was_successful) = data.as_ref().unwrap();
    assert_eq!(subject, "Post Send Test");
    assert_eq!(recipients.len(), 2);
    assert!(was_successful);

    // Verify email was actually sent
    assert_eq!(backend.count(), 1);
}

#[tokio::test]
async fn test_signal_with_failed_send() {
    let email_post_send = create_post_send_signal();

    // Track signal calls
    let failed_send = Arc::new(Mutex::new(false));
    let failed_send_clone = failed_send.clone();

    email_post_send.connect(move |data: Arc<(EmailMessage, bool)>| {
        let failed_send = failed_send_clone.clone();
        async move {
            let (_message, success) = &*data;

            if !success {
                *failed_send.lock().unwrap() = true;
            }

            Ok(())
        }
    });

    // Create a message (we'll simulate failure by not actually sending)
    let message = EmailMessage::new()
        .subject("Failed Send")
        .body("This send will fail")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    // Simulate failed send by firing signal with success=false
    email_post_send.send((message, false)).await.unwrap();

    // Verify signal detected the failure
    assert!(*failed_send.lock().unwrap());
}

/// Example of using signals in a real email sending workflow
async fn send_email_with_signals(
    message: EmailMessage,
    backend: &dyn EmailBackend,
    pre_send: &Signal<EmailMessage>,
    post_send: &Signal<(EmailMessage, bool)>,
) -> Result<(), reinhardt_mail::EmailError> {
    // Fire pre-send signal
    pre_send.send(message.clone()).await.ok();

    // Send the email
    let result = backend.send(&message).await;
    let success = result.is_ok();

    // Fire post-send signal with result
    post_send.send((message, success)).await.ok();

    result
}

#[tokio::test]
async fn test_complete_signal_workflow() {
    let email_pre_send = create_pre_send_signal();
    let email_post_send = create_post_send_signal();

    let pre_send_called = Arc::new(Mutex::new(false));
    let post_send_called = Arc::new(Mutex::new(false));

    let pre_clone = pre_send_called.clone();
    let post_clone = post_send_called.clone();

    // Connect both signals
    email_pre_send.connect(move |_| {
        let pre = pre_clone.clone();
        async move {
            *pre.lock().unwrap() = true;
            Ok(())
        }
    });

    email_post_send.connect(move |_| {
        let post = post_clone.clone();
        async move {
            *post.lock().unwrap() = true;
            Ok(())
        }
    });

    // Send email with signals
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Complete Workflow")
        .body("Testing complete signal workflow")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result =
        send_email_with_signals(message, &backend, &email_pre_send, &email_post_send).await;

    assert!(result.is_ok());
    assert!(*pre_send_called.lock().unwrap());
    assert!(*post_send_called.lock().unwrap());
    assert_eq!(backend.count(), 1);
}
