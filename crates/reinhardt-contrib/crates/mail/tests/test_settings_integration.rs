//! Settings integration tests
//!
//! Tests for Django-style settings integration with email system.

use reinhardt_mail::{
    backend_from_settings, mail_admins, mail_managers, EmailMessage, MemoryBackend, SmtpBackend,
};
use reinhardt_settings::EmailSettings;

#[tokio::test]
async fn test_backend_from_settings_smtp() {
    let mut settings = EmailSettings::default();
    settings.backend = "smtp".to_string();
    settings.host = "smtp.example.com".to_string();
    settings.port = 587;
    settings.use_tls = true;
    settings.username = Some("user".to_string());
    settings.password = Some("pass".to_string());

    let backend = backend_from_settings(&settings);

    // Verify backend was created (we can't inspect internal state directly,
    // but we can verify it's the right type by using it)
    let message = EmailMessage::new()
        .subject("Test")
        .body("Test body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    // This will fail to send (no real SMTP server), but that's expected
    // We're just verifying the backend was created correctly
    let result = backend.send(&message).await;
    assert!(result.is_err()); // Expected to fail without real server
}

#[tokio::test]
async fn test_backend_from_settings_console() {
    let mut settings = EmailSettings::default();
    settings.backend = "console".to_string();

    let backend = backend_from_settings(&settings);

    let message = EmailMessage::new()
        .subject("Console Test")
        .body("This should print to console")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_backend_from_settings_memory() {
    let mut settings = EmailSettings::default();
    settings.backend = "memory".to_string();

    let backend = backend_from_settings(&settings);

    let message = EmailMessage::new()
        .subject("Memory Test")
        .body("This should be stored in memory")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mail_admins_helper() {
    let mut settings = EmailSettings::default();
    settings.admins = vec![
        ("Admin One".to_string(), "admin1@example.com".to_string()),
        ("Admin Two".to_string(), "admin2@example.com".to_string()),
    ];
    settings.server_email = "server@example.com".to_string();

    let backend = MemoryBackend::new();

    let result = mail_admins(
        &settings,
        "Server Error",
        "Something went wrong!",
        false,
        &backend,
    )
    .await;

    assert!(result.is_ok());

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message.subject, "Server Error");
    assert_eq!(message.body, "Something went wrong!");
    assert_eq!(message.from_email, "server@example.com");
    assert_eq!(message.to.len(), 2);
    assert!(message.to.contains(&"admin1@example.com".to_string()));
    assert!(message.to.contains(&"admin2@example.com".to_string()));
}

#[tokio::test]
async fn test_mail_managers_helper() {
    let mut settings = EmailSettings::default();
    settings.managers = vec![
        (
            "Manager One".to_string(),
            "manager1@example.com".to_string(),
        ),
        (
            "Manager Two".to_string(),
            "manager2@example.com".to_string(),
        ),
    ];
    settings.server_email = "server@example.com".to_string();

    let backend = MemoryBackend::new();

    let result = mail_managers(
        &settings,
        "Important Update",
        "Please review this.",
        false,
        &backend,
    )
    .await;

    assert!(result.is_ok());

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message.subject, "Important Update");
    assert_eq!(message.body, "Please review this.");
    assert_eq!(message.from_email, "server@example.com");
    assert_eq!(message.to.len(), 2);
    assert!(message.to.contains(&"manager1@example.com".to_string()));
    assert!(message.to.contains(&"manager2@example.com".to_string()));
}

#[tokio::test]
async fn test_settings_subject_prefix() {
    let mut settings = EmailSettings::default();
    settings.admins = vec![("Admin".to_string(), "admin@example.com".to_string())];
    settings.subject_prefix = "[Django]".to_string();

    let backend = MemoryBackend::new();

    let result = mail_admins(&settings, "Error occurred", "Details here", false, &backend).await;

    assert!(result.is_ok());

    let messages = backend.get_messages();
    assert_eq!(messages[0].subject, "[Django] Error occurred");
}

#[tokio::test]
async fn test_settings_server_email() {
    let mut settings = EmailSettings::default();
    settings.admins = vec![("Admin".to_string(), "admin@example.com".to_string())];
    settings.server_email = "noreply@server.com".to_string();

    let backend = MemoryBackend::new();

    let result = mail_admins(&settings, "Test", "Body", false, &backend).await;

    assert!(result.is_ok());

    let messages = backend.get_messages();
    assert_eq!(messages[0].from_email, "noreply@server.com");
}

#[tokio::test]
async fn test_settings_timeout_configuration() {
    let mut settings = EmailSettings::default();
    settings.backend = "smtp".to_string();
    settings.timeout = Some(10);

    // Verify timeout is set in settings
    assert_eq!(settings.timeout, Some(10));

    // Create backend from settings
    let _backend = SmtpBackend::from_settings(&settings);

    // Note: We can't easily test the timeout is actually used without
    // a real SMTP connection, but we've verified the setting exists
}

#[tokio::test]
async fn test_mail_admins_empty_list_fail_silently_false() {
    let settings = EmailSettings::default(); // No admins configured
    let backend = MemoryBackend::new();

    let result = mail_admins(&settings, "Test", "Body", false, &backend).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        reinhardt_mail::EmailError::MissingField(_)
    ));
}

#[tokio::test]
async fn test_mail_admins_empty_list_fail_silently_true() {
    let settings = EmailSettings::default(); // No admins configured
    let backend = MemoryBackend::new();

    let result = mail_admins(&settings, "Test", "Body", true, &backend).await;

    assert!(result.is_ok()); // Should succeed silently
    assert_eq!(backend.count(), 0); // No email sent
}

#[tokio::test]
async fn test_mail_managers_empty_list_fail_silently_false() {
    let settings = EmailSettings::default(); // No managers configured
    let backend = MemoryBackend::new();

    let result = mail_managers(&settings, "Test", "Body", false, &backend).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        reinhardt_mail::EmailError::MissingField(_)
    ));
}

#[tokio::test]
async fn test_mail_managers_empty_list_fail_silently_true() {
    let settings = EmailSettings::default(); // No managers configured
    let backend = MemoryBackend::new();

    let result = mail_managers(&settings, "Test", "Body", true, &backend).await;

    assert!(result.is_ok()); // Should succeed silently
    assert_eq!(backend.count(), 0); // No email sent
}
