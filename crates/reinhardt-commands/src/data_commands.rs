//! Data fixture and development seeding commands.

use crate::{CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use reinhardt_conf::HasCommonSettings;
use std::path::PathBuf;
use std::sync::Arc;

/// Context passed to development seed hooks.
#[derive(Clone)]
pub struct SeedContext {
	/// Verbosity level from the CLI.
	pub verbosity: u8,
	/// Optional composed project settings.
	pub settings: Option<Arc<dyn HasCommonSettings>>,
}

impl SeedContext {
	/// Create a seed context.
	pub fn new(verbosity: u8, settings: Option<Arc<dyn HasCommonSettings>>) -> Self {
		Self {
			verbosity,
			settings,
		}
	}

	/// Convert to a management command context.
	pub fn command_context(&self) -> CommandContext {
		let ctx = CommandContext {
			verbosity: self.verbosity,
			..CommandContext::default()
		};
		match self.settings.clone() {
			Some(settings) => ctx.with_settings(settings),
			None => ctx,
		}
	}
}

/// Idempotent development data seed hook.
#[async_trait]
pub trait SeedHook: Send + Sync {
	/// Application label served by this hook.
	fn app_label(&self) -> &'static str;

	/// Run idempotent seed logic.
	async fn seed(&self, ctx: &SeedContext) -> CommandResult<()>;
}

/// Inventory registration entry for a seed hook.
pub struct SeedHookRegistration {
	/// Application label served by this hook.
	pub app_label: &'static str,
	/// Hook constructor.
	pub hook: fn() -> Box<dyn SeedHook>,
}

inventory::collect!(SeedHookRegistration);

/// Return registered seed hooks in stable app-label order.
pub fn collect_seed_hooks() -> Vec<&'static SeedHookRegistration> {
	let mut hooks = inventory::iter::<SeedHookRegistration>
		.into_iter()
		.collect::<Vec<_>>();
	hooks.sort_by_key(|hook| hook.app_label);
	hooks
}

/// Execute `manage dumpdata`.
pub async fn execute_dumpdata(
	selectors: Vec<String>,
	excludes: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
	let records = reinhardt_db::orm::fixtures::dump_fixture_records(&selectors, &excludes).await?;
	let stdout = std::io::stdout();
	let mut handle = stdout.lock();
	serde_json::to_writer_pretty(&mut handle, &records)?;
	use std::io::Write;
	writeln!(&mut handle)?;
	Ok(())
}

/// Execute `manage loaddata`.
pub async fn execute_loaddata(paths: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
	if paths.is_empty() {
		return Err(Box::new(CommandError::InvalidArguments(
			"loaddata requires at least one fixture path".to_string(),
		)));
	}

	let mut records = Vec::new();
	for path in &paths {
		let content = std::fs::read_to_string(path)?;
		let mut fixture_records: Vec<reinhardt_db::orm::fixtures::FixtureRecord> =
			serde_json::from_str(&content)?;
		records.append(&mut fixture_records);
	}

	let loaded = reinhardt_db::orm::fixtures::load_fixture_records(&records).await?;
	println!(
		"Installed {} object(s) from {} fixture file(s)",
		loaded,
		paths.len()
	);
	Ok(())
}

/// Execute `manage seed`.
pub async fn execute_seed(
	app_labels: Vec<String>,
	verbosity: u8,
	settings: Option<Arc<dyn HasCommonSettings>>,
) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = SeedContext::new(verbosity, settings);
	let requested = app_labels
		.into_iter()
		.collect::<std::collections::HashSet<_>>();
	let registrations = collect_seed_hooks();
	let registered = registrations
		.iter()
		.map(|registration| registration.app_label)
		.collect::<std::collections::HashSet<_>>();
	let mut unknown = requested
		.iter()
		.filter(|label| !registered.contains(label.as_str()))
		.cloned()
		.collect::<Vec<_>>();
	unknown.sort();
	if !unknown.is_empty() {
		return Err(Box::new(CommandError::InvalidArguments(format!(
			"no seed hooks registered for {}",
			unknown.join(", ")
		))));
	}
	let hooks = registrations
		.into_iter()
		.filter(|registration| requested.is_empty() || requested.contains(registration.app_label))
		.collect::<Vec<_>>();

	for registration in hooks {
		let hook = (registration.hook)();
		hook.seed(&ctx).await?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	struct ExampleSeedHook;

	#[async_trait]
	impl SeedHook for ExampleSeedHook {
		fn app_label(&self) -> &'static str {
			"example"
		}

		async fn seed(&self, ctx: &SeedContext) -> CommandResult<()> {
			assert_eq!(ctx.verbosity, 2);
			Ok(())
		}
	}

	#[tokio::test]
	async fn seed_hook_receives_context() {
		let hook = ExampleSeedHook;
		let ctx = SeedContext::new(2, None);

		hook.seed(&ctx).await.unwrap();
	}

	inventory::submit! {
		SeedHookRegistration {
			app_label: "example",
			hook: || Box::new(ExampleSeedHook),
		}
	}

	#[tokio::test]
	async fn seed_rejects_unknown_labels_even_when_other_labels_match() {
		let error = execute_seed(vec!["example".to_string(), "missing".to_string()], 0, None)
			.await
			.unwrap_err();

		assert_eq!(
			error.to_string(),
			"Invalid arguments: no seed hooks registered for missing"
		);
	}
}
