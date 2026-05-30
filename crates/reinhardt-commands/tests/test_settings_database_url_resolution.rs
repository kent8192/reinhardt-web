//! Regression test for issue #5042.
//!
//! Database-requiring management commands (`migrate`, `makemigrations`,
//! `runserver`, `createsuperuser`) must resolve the database connection from a
//! project's composed settings (`[core.databases.default]`) when `DATABASE_URL`
//! is not set. Before #5042 the command runtime built `CommandContext` without
//! any settings, so the settings-aware resolution arm of
//! `initialize_orm_database` was unreachable and the only working path was the
//! `DATABASE_URL` environment variable.
//!
//! This test pins the behaviour the fix makes reachable: given a
//! `CommandContext` carrying composed settings with a `[core.databases.default]`
//! block, `DatabaseConnection::database_url_from(ctx.settings, None)` resolves
//! the connection URL without consulting any environment variable (the `None`
//! `env_override` argument means "do not override from the environment").

#![cfg(feature = "reinhardt-db")]

use reinhardt_commands::CommandContext;
use reinhardt_conf::HasCommonSettings;
use reinhardt_conf::settings::DatabaseConfig;
use reinhardt_conf::settings::contacts::ContactSettings;
use reinhardt_conf::settings::core_settings::CoreSettings;
use reinhardt_conf::settings::fragment::HasSettings;
use reinhardt_db::backends::DatabaseConnection;
use rstest::rstest;
use std::collections::HashMap;
use std::sync::Arc;

/// Minimal stub satisfying [`HasCommonSettings`] without going through the
/// `#[settings(...)]` proc-macro, mirroring the pattern used in
/// `test_sendtestemail.rs`. A real project would hand `with_settings()` the
/// `ProjectSettings` value returned by its own `get_settings()`.
struct StubProjectSettings {
	core: CoreSettings,
	contacts: ContactSettings,
}

impl HasSettings<CoreSettings> for StubProjectSettings {
	fn get_settings(&self) -> &CoreSettings {
		&self.core
	}
}

impl HasSettings<ContactSettings> for StubProjectSettings {
	fn get_settings(&self) -> &ContactSettings {
		&self.contacts
	}
}

/// Build composed settings whose `[core.databases.default]` entry points at a
/// PostgreSQL database, exactly as a project's `settings/base.toml` would.
fn settings_with_postgres_default() -> Arc<dyn HasCommonSettings> {
	let mut databases = HashMap::new();
	databases.insert(
		"default".to_string(),
		DatabaseConfig::postgresql("myapp", "reinhardt", "reinhardt", "localhost", 5432),
	);

	Arc::new(StubProjectSettings {
		core: CoreSettings {
			secret_key: "stub-secret".to_string(),
			databases,
			..Default::default()
		},
		contacts: ContactSettings::default(),
	})
}

#[rstest]
fn settings_context_resolves_database_url_without_env_override() {
	// Arrange: a command context carrying the project's composed settings,
	// as the runtime now builds for DB-requiring commands (#5042).
	let settings = settings_with_postgres_default();
	let ctx = CommandContext::new(vec![]).with_settings(settings);

	// Act: resolve exactly as `initialize_orm_database`'s now-reachable
	// `Some(settings)` arm does — `None` means "no environment override".
	let resolved = DatabaseConnection::database_url_from(
		ctx.settings
			.as_ref()
			.expect("settings were attached to the context")
			.as_ref(),
		None,
	)
	.expect("settings-based database URL resolution should succeed");

	// Assert: the URL comes straight from `[core.databases.default]`.
	assert_eq!(
		resolved,
		"postgresql://reinhardt:reinhardt@localhost:5432/myapp"
	);
}

#[rstest]
fn context_without_settings_leaves_settings_none() {
	// Arrange & Act: the no-settings constructor still yields an empty handle,
	// documenting that the settings-aware arm is only taken when a project
	// hands its settings to the runtime.
	let ctx = CommandContext::new(vec![]);

	// Assert
	assert!(ctx.settings.is_none());
}
