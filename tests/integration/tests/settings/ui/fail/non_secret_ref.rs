// This compile-fail fixture defines settings types only to exercise generated
// schema refs.
#![allow(dead_code)]

use reinhardt_conf::settings::schema::{HasSettingsSchema, SecretFieldRef};
use reinhardt_conf::settings::secret_types::SecretString;
use reinhardt_macros::settings;

#[settings(fragment = true, section = "compile_fail_database_config", default_policy = "required")]
struct CompileFailDatabaseConfig {
	pub host: String,
	pub password: SecretString,
}

#[settings(fragment = true, section = "database")]
struct CompileFailDatabaseSettings {
	#[setting(required)]
	pub default: CompileFailDatabaseConfig,
}

#[settings(database: CompileFailDatabaseSettings)]
struct CompileFailProjectSettings;

fn accepts_secret(_: SecretFieldRef<CompileFailProjectSettings, SecretString>) {}

fn main() {
	let schema = CompileFailProjectSettings::schema();
	accepts_secret(schema.database.default.host);
}
