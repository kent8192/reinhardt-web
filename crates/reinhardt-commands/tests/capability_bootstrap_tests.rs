#![cfg(feature = "contract")]

use async_trait::async_trait;
use reinhardt_commands::capabilities::PreparedValue;
use reinhardt_commands::capabilities::{
	CheckInputs, CoreMigrationMetadata, LocalInfrastructureSettings,
};
use reinhardt_commands::{
	CapabilityContext, CapabilityProvider, CapabilityRequirement, CommandError, CommandResult,
	SelectedDatabase, SettingsView,
};
use reinhardt_conf::settings::PendingSettings;
use reinhardt_conf::settings::builder::{BuildError, SettingsBuilder};
use reinhardt_conf::settings::scoped::ScopedSettings;
use reinhardt_conf::settings::sources::{DefaultSource, TomlFileSource};
use reinhardt_core::macros::settings;
use rstest::*;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[settings(core: CoreSettings | contacts: ContactSettings | migrations: MigrationSettings)]
struct ProjectSettings;

struct ExampleView(String);

impl SettingsView for ExampleView {
	const NAME: &'static str = "example view";

	fn resolve(settings: &ScopedSettings, _alias: Option<&str>) -> Result<Self, BuildError> {
		settings
			.require_path::<String>(&["example", "value"])
			.map(Self)
	}
}

struct Guard(Arc<AtomicUsize>);

impl Drop for Guard {
	fn drop(&mut self) {
		self.0.fetch_add(1, Ordering::SeqCst);
	}
}

struct Provider {
	scoped_calls: Arc<AtomicUsize>,
	service_calls: Arc<AtomicUsize>,
	drops: Arc<AtomicUsize>,
}

#[async_trait]
impl CapabilityProvider for Provider {
	type Settings = ProjectSettings;

	fn scoped_settings(&self) -> Result<ScopedSettings, BuildError> {
		self.scoped_calls.fetch_add(1, Ordering::SeqCst);
		SettingsBuilder::new()
			.add_source(DefaultSource::new().with_value("example", json!({"value": "ready"})))
			.build_scoped()
	}

	fn full_settings(&self) -> Result<PendingSettings<ProjectSettings>, BuildError> {
		Err(BuildError::Deserialization(
			"full settings must not be requested for scoped command".to_owned(),
		))
	}

	async fn prepare_service(
		&self,
		requirement: &CapabilityRequirement,
		context: &CapabilityContext,
	) -> CommandResult<PreparedValue> {
		assert_eq!(context.settings::<ExampleView>(None)?.0, "ready");
		self.service_calls.fetch_add(1, Ordering::SeqCst);
		if requirement.alias() == Some("fail") {
			return Err(CommandError::ExecutionError("service failed".to_owned()));
		}
		Ok(Arc::new(Guard(self.drops.clone())))
	}
}

fn provider() -> Provider {
	Provider {
		scoped_calls: Arc::new(AtomicUsize::new(0)),
		service_calls: Arc::new(AtomicUsize::new(0)),
		drops: Arc::new(AtomicUsize::new(0)),
	}
}

#[rstest]
#[tokio::test]
async fn prepares_once_per_type_and_alias_then_drops_with_invocation() {
	let provider = provider();
	let requirements = [
		CapabilityRequirement::settings::<ExampleView>(None),
		CapabilityRequirement::settings::<ExampleView>(None),
		CapabilityRequirement::service::<Guard>("guard", None),
		CapabilityRequirement::service::<Guard>("guard", None),
	];
	let context = CapabilityContext::prepare("example", &requirements, &provider)
		.await
		.unwrap();
	assert_eq!(context.settings::<ExampleView>(None).unwrap().0, "ready");
	assert!(context.service::<Guard>("guard", None).is_ok());
	assert!(context.service::<Guard>("guard", Some("other")).is_err());
	assert_eq!(provider.scoped_calls.load(Ordering::SeqCst), 1);
	assert_eq!(provider.service_calls.load(Ordering::SeqCst), 1);
	drop(context);
	assert_eq!(provider.drops.load(Ordering::SeqCst), 1);
}

#[rstest]
#[tokio::test]
async fn partial_preparation_failure_releases_prepared_resources() {
	let provider = provider();
	let requirements = [
		CapabilityRequirement::settings::<ExampleView>(None),
		CapabilityRequirement::service::<Guard>("guard", None),
		CapabilityRequirement::service::<Guard>("guard", Some("fail")),
	];
	assert!(
		CapabilityContext::prepare("example", &requirements, &provider)
			.await
			.is_err()
	);
	assert_eq!(provider.service_calls.load(Ordering::SeqCst), 2);
	assert_eq!(provider.drops.load(Ordering::SeqCst), 1);
}

#[rstest]
fn database_view_reads_only_selected_alias() {
	let dir = tempfile::tempdir().unwrap();
	let config = dir.path().join("settings.toml");
	std::fs::write(&config, "[core]\nsecret_key = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\n[core.databases.default]\nengine = \"sqlite\"\nname = \"db.sqlite3\"\n[core.databases.other]\nengine = \"postgresql\"\nname = \"other\"\npassword = \"${REINHARDT_SCOPED_MISSING_DB_PASSWORD_6336}\"\n").unwrap();
	let scoped = SettingsBuilder::new()
		.add_source(TomlFileSource::new(config))
		.build_scoped()
		.unwrap();
	let selected = SelectedDatabase::resolve(&scoped, Some("default")).unwrap();
	assert_eq!(selected.alias(), "default");
	assert_eq!(selected.url(), "sqlite:db.sqlite3");
	assert!(SelectedDatabase::resolve(&scoped, Some("other")).is_err());
	let sensitive_alias = "postgres://user:password@host/database";
	let error = SelectedDatabase::resolve(&scoped, Some(sensitive_alias))
		.err()
		.expect("URL-shaped alias must be rejected")
		.to_string();
	assert!(!error.contains("password"));
}

#[rstest]
fn migration_metadata_skips_unrelated_runtime_secrets() {
	let dir = tempfile::tempdir().unwrap();
	let config = dir.path().join("settings.toml");
	std::fs::write(
		&config,
		"[core]\nsecret_key = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\ninstalled_apps = [\"accounts\"]\nmigration_features = [\"audit\"]\n[core.migration_swappable_settings]\nAUTH_USER_MODEL = \"accounts.User\"\n",
	).unwrap();
	let scoped = SettingsBuilder::new()
		.add_source(TomlFileSource::new(config))
		.build_scoped()
		.unwrap();
	let metadata = CoreMigrationMetadata::resolve(&scoped, None).unwrap();
	assert_eq!(metadata.installed_apps, ["accounts"]);
	assert_eq!(metadata.migration_features, ["audit"]);
	assert_eq!(
		metadata
			.migration_swappable_settings
			.get("AUTH_USER_MODEL")
			.map(String::as_str),
		Some("accounts.User")
	);
}

#[rstest]
fn local_infrastructure_only_resolves_postgresql_inputs() {
	let dir = tempfile::tempdir().unwrap();
	let config = dir.path().join("settings.toml");
	std::fs::write(
		&config,
		"[core]\nsecret_key = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\n[core.databases.default]\nengine = \"sqlite\"\nname = \"local.sqlite3\"\npassword = \"${REINHARDT_SCOPED_MISSING_DB_PASSWORD_6336}\"\n",
	).unwrap();
	let scoped = SettingsBuilder::new()
		.add_source(TomlFileSource::new(config))
		.build_scoped()
		.unwrap();
	assert!(
		LocalInfrastructureSettings::resolve(&scoped, None)
			.unwrap()
			.database
			.is_none()
	);
}

#[rstest]
fn ordinary_check_does_not_resolve_deployment_secret() {
	let dir = tempfile::tempdir().unwrap();
	let config = dir.path().join("settings.toml");
	std::fs::write(
		&config,
		"[core]\nsecret_key = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\n[static]\nroot = \"dist\"\n",
	)
	.unwrap();
	let scoped = SettingsBuilder::new()
		.add_source(TomlFileSource::new(config))
		.build_scoped()
		.unwrap();
	assert!(CheckInputs::resolve(&scoped, None).is_ok());
	assert!(CheckInputs::resolve(&scoped, Some("deploy")).is_err());
}
