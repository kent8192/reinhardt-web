use reinhardt::settings;
use reinhardt::prelude::*;
use reinhardt::conf::settings::builder::SettingsBuilder;
use reinhardt::conf::settings::sources::{DefaultSource, TomlFileSource};

#[settings(core: CoreSettings | contacts: ContactSettings | migrations: MigrationSettings)]
pub struct ProjectSettings;

pub fn settings_builder() -> SettingsBuilder {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    SettingsBuilder::new()
        .add_source(DefaultSource::new()
            .with_value("core", serde_json::json!({"base_dir": root, "installed_apps": []}))
            .with_value("contacts", serde_json::json!({}))
            .with_value("migrations", serde_json::json!({})))
        .add_source(TomlFileSource::new(root.join("settings/base.toml")))
}
