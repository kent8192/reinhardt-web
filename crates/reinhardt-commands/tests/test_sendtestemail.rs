//! Tests for sendtestemail command
//! Based on Django's django/tests/mail/test_sendtestemail.py

use reinhardt_commands::{BaseCommand, CommandContext, SendTestEmailCommand};
use reinhardt_conf::HasCommonSettings;
use reinhardt_conf::settings::Contact;
use reinhardt_conf::settings::contacts::ContactSettings;
use reinhardt_conf::settings::core_settings::CoreSettings;
use reinhardt_conf::settings::fragment::HasSettings;
use std::sync::Arc;

/// Minimal stub satisfying [`HasCommonSettings`] without going through
/// the `#[settings(...)]` proc-macro. Exists solely so the legacy mail
/// tests can hand `with_settings()` an `Arc<dyn HasCommonSettings>`
/// after the `CommandContext.settings` field was lifted off the
/// deprecated `Settings` type (PR-A of issues #4277 / #4282).
struct StubMailSettings {
	core: CoreSettings,
	contacts: ContactSettings,
}

impl HasSettings<CoreSettings> for StubMailSettings {
	fn get_settings(&self) -> &CoreSettings {
		&self.core
	}
}

impl HasSettings<ContactSettings> for StubMailSettings {
	fn get_settings(&self) -> &ContactSettings {
		&self.contacts
	}
}

fn settings_with_contacts(
	admins: Vec<Contact>,
	managers: Vec<Contact>,
) -> Arc<dyn HasCommonSettings> {
	Arc::new(StubMailSettings {
		core: CoreSettings {
			secret_key: "stub-secret".to_string(),
			..Default::default()
		},
		contacts: ContactSettings { admins, managers },
	})
}

#[tokio::test]
async fn test_single_receiver() {
	let command = SendTestEmailCommand::new();
	let ctx = CommandContext::new(vec!["joe@example.com".to_string()]);

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_sendtestemail_multiple_receivers() {
	let command = SendTestEmailCommand::new();
	let ctx = CommandContext::new(vec![
		"joe@example.com".to_string(),
		"jane@example.com".to_string(),
	]);

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_missing_receivers() {
	let command = SendTestEmailCommand::new();
	let ctx = CommandContext::new(vec![]);

	let result = command.execute(&ctx).await;
	assert!(result.is_err());

	if let Err(e) = result {
		assert!(
			e.to_string()
				.contains("You must specify some email recipients")
				|| e.to_string()
					.contains("or pass the --managers or --admin options")
		);
	}
}

#[tokio::test]
async fn test_manager_receivers() {
	let command = SendTestEmailCommand::new();

	// Create mock settings with manager contacts
	let settings =
		settings_with_contacts(vec![], vec![Contact::new("Manager", "manager@example.com")]);

	let mut ctx = CommandContext::new(vec![]).with_settings(settings);
	ctx.set_option("managers".to_string(), "true".to_string());

	let result = command.execute(&ctx).await;
	// Should succeed since --managers flag is set
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_admin_receivers() {
	let command = SendTestEmailCommand::new();

	// Create mock settings with admin contacts
	let settings = settings_with_contacts(vec![Contact::new("Admin", "admin@example.com")], vec![]);

	let mut ctx = CommandContext::new(vec![]).with_settings(settings);
	ctx.set_option("admins".to_string(), "true".to_string());

	let result = command.execute(&ctx).await;
	// Should succeed since --admins flag is set
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_manager_and_admin_receivers() {
	let command = SendTestEmailCommand::new();

	// Create mock settings with both manager and admin contacts
	let settings = settings_with_contacts(
		vec![Contact::new("Admin", "admin@example.com")],
		vec![Contact::new("Manager", "manager@example.com")],
	);

	let mut ctx = CommandContext::new(vec![]).with_settings(settings);
	ctx.set_option("managers".to_string(), "true".to_string());
	ctx.set_option("admins".to_string(), "true".to_string());

	let result = command.execute(&ctx).await;
	// Should succeed since both --managers and --admins flags are set
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_backend_console() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec!["test@example.com".to_string()]);
	ctx.set_option("backend".to_string(), "console".to_string());

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_backend_memory() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec!["test@example.com".to_string()]);
	ctx.set_option("backend".to_string(), "memory".to_string());

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_backend_file() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec!["test@example.com".to_string()]);
	ctx.set_option("backend".to_string(), "file".to_string());

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_verbose_output() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec![
		"test1@example.com".to_string(),
		"test2@example.com".to_string(),
	]);
	ctx.set_option("verbose".to_string(), "true".to_string());
	ctx.set_option("backend".to_string(), "memory".to_string());

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_settings_file_option() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec!["test@example.com".to_string()]);
	ctx.set_option(
		"settings".to_string(),
		"/tmp/test_settings.toml".to_string(),
	);
	ctx.set_option("backend".to_string(), "console".to_string());

	let result = command.execute(&ctx).await;
	// Should succeed even if file doesn't exist (we use defaults)
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_backend_and_verbose_combined() {
	let command = SendTestEmailCommand::new();
	let mut ctx = CommandContext::new(vec!["test@example.com".to_string()]);
	ctx.set_option("backend".to_string(), "memory".to_string());
	ctx.set_option("verbose".to_string(), "true".to_string());

	let result = command.execute(&ctx).await;
	assert!(result.is_ok());
}
