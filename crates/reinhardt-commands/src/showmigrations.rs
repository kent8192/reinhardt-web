//! Migration metadata handoff for capability-aware command dispatch.

use crate::CommandContext;
#[cfg(feature = "contract")]
use reinhardt_conf::MigrationSettings;

#[cfg(feature = "contract")]
const MIGRATION_FEATURES_OPTION: &str = "__reinhardt_migration_features";
#[cfg(feature = "contract")]
const MIGRATION_SETTING_OPTION_PREFIX: &str = "__reinhardt_migration_setting:";
#[cfg(feature = "contract")]
const CORE_MIGRATION_APPS_OPTION: &str = "__reinhardt_core_migration_apps";
#[cfg(feature = "contract")]
const CORE_MIGRATION_FEATURES_OPTION: &str = "__reinhardt_core_migration_features";
#[cfg(feature = "contract")]
const CORE_MIGRATION_SETTING_OPTION_PREFIX: &str = "__reinhardt_core_migration_setting:";
#[cfg(feature = "contract")]
const CORE_MIGRATION_BASE_DIR_OPTION: &str = "__reinhardt_core_migration_base_dir";

#[cfg(feature = "contract")]
pub(crate) fn attach_core_migration_metadata(
	ctx: &mut CommandContext,
	metadata: &crate::capabilities::CoreMigrationMetadata,
) {
	ctx.set_option_multi(
		CORE_MIGRATION_APPS_OPTION.to_owned(),
		metadata.installed_apps.clone(),
	);
	ctx.set_option_multi(
		CORE_MIGRATION_FEATURES_OPTION.to_owned(),
		metadata.migration_features.clone(),
	);
	ctx.set_option(
		CORE_MIGRATION_BASE_DIR_OPTION.to_owned(),
		metadata.base_dir.to_string_lossy().into_owned(),
	);
	for (key, value) in &metadata.migration_swappable_settings {
		ctx.set_option(
			format!("{CORE_MIGRATION_SETTING_OPTION_PREFIX}{key}"),
			value.clone(),
		);
	}
}

pub(crate) fn migration_installed_apps(ctx: &CommandContext) -> Option<&[String]> {
	#[cfg(feature = "contract")]
	if let Some(apps) = ctx.options.get(CORE_MIGRATION_APPS_OPTION) {
		return Some(apps);
	}
	ctx.settings
		.as_ref()
		.map(|settings| settings.core().installed_apps.as_slice())
}

#[cfg(feature = "contract")]
pub(crate) fn attach_migration_settings(ctx: &mut CommandContext, settings: &MigrationSettings) {
	ctx.set_option_multi(
		MIGRATION_FEATURES_OPTION.to_string(),
		settings.migration_features.clone(),
	);
	for (key, value) in settings
		.migration_settings
		.iter()
		.chain(&settings.migration_swappable_settings)
	{
		ctx.set_option(
			format!("{MIGRATION_SETTING_OPTION_PREFIX}{key}"),
			value.clone(),
		);
	}
}
