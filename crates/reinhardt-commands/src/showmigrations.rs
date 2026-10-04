//! Migration metadata handoff for capability-aware command dispatch.

use crate::CommandContext;
use async_trait::async_trait;
#[cfg(feature = "contract")]
use reinhardt_conf::MigrationSettings;
use reinhardt_db::migrations::{
	DependencyResolutionContext, DependencyResolver, FilesystemSource, Migration,
	MigrationDependency, MigrationSource,
};
use std::path::Path;

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

fn migration_dependency_context(ctx: &CommandContext) -> DependencyResolutionContext {
	let mut context = DependencyResolutionContext::new();
	for app in migration_installed_apps(ctx).unwrap_or_default() {
		context.installed_apps.insert(app.clone());
		// Installed app module paths must also satisfy app-label conditions.
		for registration in inventory::iter::<reinhardt_apps::registry::InstalledAppRegistration> {
			if registration.path == app {
				context.installed_apps.insert(registration.label.to_owned());
			}
		}
	}
	#[cfg(feature = "contract")]
	{
		for option in [CORE_MIGRATION_FEATURES_OPTION, MIGRATION_FEATURES_OPTION] {
			context.features.extend(
				ctx.option_values(option)
					.unwrap_or_default()
					.iter()
					.cloned(),
			);
		}
		// Dedicated migration settings override the core defaults.
		for prefix in [
			CORE_MIGRATION_SETTING_OPTION_PREFIX,
			MIGRATION_SETTING_OPTION_PREFIX,
		] {
			for (option, values) in &ctx.options {
				if let Some(key) = option.strip_prefix(prefix)
					&& let Some(value) = values.first()
				{
					context
						.swappable_settings
						.insert(key.to_owned(), value.clone());
				}
			}
		}
	}
	context
}

/// Resolve conditional dependencies before the stable engine consumes a source.
/// The executor, plan, conflict checks, and state loaders all receive the same
/// concrete dependency tuples without changing their existing public APIs.
pub(crate) struct CommandMigrationSource {
	source: FilesystemSource,
	context: DependencyResolutionContext,
}

impl CommandMigrationSource {
	pub(crate) fn new(path: &Path, ctx: &CommandContext) -> Self {
		Self {
			source: FilesystemSource::new(path),
			context: migration_dependency_context(ctx),
		}
	}
}

#[async_trait]
impl MigrationSource for CommandMigrationSource {
	async fn all_migrations(&self) -> reinhardt_db::migrations::Result<Vec<Migration>> {
		let mut migrations = self.source.all_migrations().await?;
		let mut context = self.context.clone();
		if context.installed_apps.is_empty() {
			// Empty installed-app defaults retain automatic discovery.
			context.installed_apps.extend(
				migrations
					.iter()
					.map(|migration| migration.app_label.clone()),
			);
		}
		let resolver = DependencyResolver::new(&context);
		for migration in &mut migrations {
			let dependencies = migration
				.swappable_dependencies
				.iter()
				.cloned()
				.map(MigrationDependency::Swappable)
				.chain(
					migration
						.optional_dependencies
						.iter()
						.cloned()
						.map(MigrationDependency::Optional),
				);
			for dependency in dependencies {
				if let Some(resolved) = resolver.resolve(&dependency)
					&& !migration.dependencies.contains(&resolved)
				{
					migration.dependencies.push(resolved);
				}
			}
		}
		Ok(migrations)
	}
}
