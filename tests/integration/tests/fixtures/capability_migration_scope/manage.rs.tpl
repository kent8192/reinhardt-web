use reinhardt::commands::capabilities::CoreMigrationMetadata;
use reinhardt::commands::{
	CapabilityProvider, CommandRegistry, SettingsView, execute_from_command_line_with_capabilities,
};
use reinhardt::conf::settings::{
	PendingSettings,
	builder::{BuildError, SettingsBuilder},
	scoped::ScopedSettings,
	sources::DefaultSource,
};
use reinhardt::db::migrations::model_registry::global_registry;
use reinhardt::db::orm::Model;
use reinhardt::{installed_apps, model, settings};

installed_apps! { identity: "consumer.accounts" }

#[model(app_label = "identity", table_name = "own_identities")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Identity {
	#[field(primary_key = true)]
	pub id: i64,
}

#[settings(core: CoreSettings | contacts: ContactSettings | migrations: MigrationSettings)]
pub struct Settings;

struct Provider;

impl CapabilityProvider for Provider {
	type Settings = Settings;

	fn scoped_settings(&self) -> Result<ScopedSettings, BuildError> {
		let installed_apps: Vec<String> = serde_json::from_str(
			&std::env::var("REINHARDT_TEST_INSTALLED_APPS").expect("test supplies installed apps"),
		)
		.expect("installed apps are a JSON array");
		let config: serde_json::Value = serde_json::from_str(
			&std::env::var("REINHARDT_TEST_MIGRATION_CONFIG").unwrap_or_else(|_| "{}".to_owned()),
		)
		.expect("migration config is JSON");
		let mut core = serde_json::json!({
			"base_dir": std::env::current_dir().unwrap(),
			"installed_apps": installed_apps,
			"secret_key": "${REINHARDT_MIGRATION_UNRELATED_SECRET}",
		});
		if let Some(overrides) = config.get("core").and_then(serde_json::Value::as_object) {
			core.as_object_mut().unwrap().extend(overrides.clone());
		}
		let settings = SettingsBuilder::new()
			.add_source(
				DefaultSource::new().with_value("core", core).with_value(
					"migrations",
					config
						.get("migrations")
						.cloned()
						.unwrap_or(serde_json::json!({})),
				),
			)
			.build_scoped()?;
		let metadata = CoreMigrationMetadata::resolve(&settings, None)?;
		assert_eq!(metadata.installed_apps, installed_apps);
		Ok(settings)
	}

	fn full_settings(&self) -> Result<PendingSettings<Settings>, BuildError> {
		panic!("file-based migration discovery must not require full runtime settings");
	}
}

fn registered_models() -> Vec<(String, String)> {
	let mut models = global_registry()
		.get_models()
		.into_iter()
		.map(|model| (model.app_label, model.table_name))
		.collect::<Vec<_>>();
	models.sort();
	models
}

#[tokio::main]
async fn main() {
	std::hint::black_box(reinhardt::auth::sessions::backends::database::Session::objects());
	std::hint::black_box(Identity::objects());
	let registered = registered_models();
	for (app, table) in [
		("auth", "auth_group"),
		("auth", "auth_permission"),
		("default", "sessions"),
		("identity", "own_identities"),
	] {
		assert_eq!(
			registered
				.iter()
				.filter(|(label, name)| label == app && name == table)
				.count(),
			1
		);
	}
	let result =
		execute_from_command_line_with_capabilities(CommandRegistry::new(), Provider, None).await;
	assert_eq!(
		registered_models(),
		registered,
		"migration discovery must preserve linked model metadata"
	);
	if let Err(error) = result {
		eprintln!("{error}");
		std::process::exit(1);
	}
}
