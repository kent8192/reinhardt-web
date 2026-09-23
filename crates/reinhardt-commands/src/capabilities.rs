//! Opt-in command capability preparation and invocation-owned resources.

use crate::{CommandError, CommandResult};
use async_trait::async_trait;
use clap::{ArgMatches, Command};
use reinhardt_conf::settings::builder::{BuildError, SettingsBuilder};
use reinhardt_conf::settings::database_config::DatabaseConfig;
use reinhardt_conf::settings::fragment::HasSettings;
use reinhardt_conf::settings::fragment::SettingsFragment;
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::scoped::ScopedSettings;
use reinhardt_conf::settings::{ComposedSettings, PendingSettings};
use reinhardt_conf::{HasCommonSettings, MigrationSettings};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// A type-erased invocation-owned resource returned by an application provider.
pub type PreparedValue = Arc<dyn Any + Send + Sync>;
type ResolveSettings = fn(&ScopedSettings, Option<&str>) -> Result<PreparedValue, BuildError>;

/// A validated view of only the configuration paths a command needs.
pub trait SettingsView: Any + Send + Sync + Sized {
	/// Safe name shown in missing-capability diagnostics.
	const NAME: &'static str;

	/// Resolve the selected values and run this view's semantic validation.
	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError>;
}

/// A typed settings or service requirement, optionally addressed by alias.
#[derive(Clone)]
pub struct CapabilityRequirement {
	kind: CapabilityKind,
	type_id: TypeId,
	name: &'static str,
	alias: Option<String>,
	resolve: Option<ResolveSettings>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CapabilityKind {
	Settings,
	Service,
}

impl CapabilityRequirement {
	/// Request a typed, validated settings view.
	pub fn settings<T: SettingsView>(alias: Option<&str>) -> Self {
		Self {
			kind: CapabilityKind::Settings,
			type_id: TypeId::of::<T>(),
			name: T::NAME,
			alias: alias.map(str::to_owned),
			resolve: Some(|settings, alias| {
				T::resolve(settings, alias).map(|value| Arc::new(value) as PreparedValue)
			}),
		}
	}

	/// Request an application service owned for the duration of the invocation.
	pub fn service<T: Any + Send + Sync>(name: &'static str, alias: Option<&str>) -> Self {
		Self {
			kind: CapabilityKind::Service,
			type_id: TypeId::of::<T>(),
			name,
			alias: alias.map(str::to_owned),
			resolve: None,
		}
	}

	/// Safe human-readable capability name.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Selected service or settings alias.
	pub fn alias(&self) -> Option<&str> {
		self.alias.as_deref()
	}

	/// Whether this requirement names the given service type.
	pub fn is_service<T: Any + Send + Sync>(&self) -> bool {
		self.kind == CapabilityKind::Service && self.type_id == TypeId::of::<T>()
	}

	fn key(&self) -> CapabilityKey {
		CapabilityKey {
			kind: self.kind,
			type_id: self.type_id,
			alias: self.alias.clone(),
		}
	}
}

/// One validated database alias without resolving unrelated aliases or secrets.
pub struct SelectedDatabase {
	alias: String,
	config: DatabaseConfig,
}

impl SelectedDatabase {
	/// The selected alias.
	pub fn alias(&self) -> &str {
		&self.alias
	}

	/// Connection URL for the selected database. Do not print this value.
	pub fn url(&self) -> String {
		self.config.to_url()
	}
}

impl SettingsView for SelectedDatabase {
	const NAME: &'static str = "database settings";

	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError> {
		let alias = alias.unwrap_or("default");
		let config = settings.require_path::<DatabaseConfig>(&["core", "databases", alias])?;
		if config.name.is_empty() {
			return Err(BuildError::Deserialization(format!(
				"database alias `{alias}` requires a non-empty `name`"
			)));
		}
		let engine = config.engine.as_str();
		if !matches!(engine, "sqlite" | "postgres" | "postgresql" | "mysql")
			&& !matches!(
				engine,
				"reinhardt.db.backends.sqlite3"
					| "reinhardt.db.backends.postgresql"
					| "reinhardt.db.backends.mysql"
			) {
			return Err(BuildError::Deserialization(format!(
				"database alias `{alias}` has an unsupported engine at `core.databases.{alias}.engine`"
			)));
		}
		Ok(Self {
			alias: alias.to_owned(),
			config,
		})
	}
}

impl SettingsView for MigrationSettings {
	const NAME: &'static str = "migration settings";

	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError> {
		if alias.is_some() {
			return Err(BuildError::Deserialization(
				"migration settings do not accept a database alias".to_owned(),
			));
		}
		let value = settings
			.optional_path::<Self>(&["migrations"])?
			.unwrap_or_default();
		let profile = settings.profile().unwrap_or(Profile::Development);
		value.validate(&profile).map_err(|_| {
			BuildError::Deserialization("invalid migration settings at `migrations`".to_owned())
		})?;
		Ok(value)
	}
}

#[derive(PartialEq, Eq, Hash)]
struct CapabilityKey {
	kind: CapabilityKind,
	type_id: TypeId,
	alias: Option<String>,
}

/// Values prepared before the command body and dropped with its invocation.
pub struct CapabilityContext {
	command: String,
	values: HashMap<CapabilityKey, PreparedValue>,
}

impl CapabilityContext {
	/// Access a declared settings view. This never initializes new capabilities.
	pub fn settings<T: SettingsView>(&self, alias: Option<&str>) -> CommandResult<Arc<T>> {
		self.get::<T>(CapabilityKind::Settings, T::NAME, alias)
	}

	/// Access a declared service. This never initializes new capabilities.
	pub fn service<T: Any + Send + Sync>(
		&self,
		name: &'static str,
		alias: Option<&str>,
	) -> CommandResult<Arc<T>> {
		self.get::<T>(CapabilityKind::Service, name, alias)
	}

	fn get<T: Any + Send + Sync>(
		&self,
		kind: CapabilityKind,
		name: &'static str,
		alias: Option<&str>,
	) -> CommandResult<Arc<T>> {
		let key = CapabilityKey {
			kind,
			type_id: TypeId::of::<T>(),
			alias: alias.map(str::to_owned),
		};
		let value = self.values.get(&key).ok_or_else(|| {
			CommandError::ExecutionError(format!(
				"command `{}` did not declare capability `{name}`{}; add it to requirements",
				self.command,
				alias
					.map(|value| format!(" (alias `{value}`)"))
					.unwrap_or_default()
			))
		})?;
		Arc::downcast::<T>(value.clone()).map_err(|_| {
			CommandError::ExecutionError(format!(
				"provider supplied the wrong type for capability `{name}` in command `{}`",
				self.command
			))
		})
	}

	/// Validate all views before initializing services, then prepare services in
	/// the application's declared order. Each type/alias pair is prepared once.
	pub async fn prepare<P: CapabilityProvider + Sync>(
		command: &str,
		requirements: &[CapabilityRequirement],
		provider: &P,
	) -> CommandResult<Self> {
		let mut context = Self {
			command: command.to_owned(),
			values: HashMap::new(),
		};
		let scoped = if requirements
			.iter()
			.any(|req| req.kind == CapabilityKind::Settings)
		{
			Some(provider.scoped_settings().map_err(|error| {
				CommandError::ExecutionError(format!(
					"command `{command}` could not load scoped settings: {error}"
				))
			})?)
		} else {
			None
		};
		for requirement in requirements
			.iter()
			.filter(|req| req.kind == CapabilityKind::Settings)
		{
			let key = requirement.key();
			if context.values.contains_key(&key) {
				continue;
			}
			let resolve = requirement
				.resolve
				.expect("settings requirement has resolver");
			let value = resolve(
				scoped.as_ref().expect("settings loaded"),
				requirement.alias(),
			)
			.map_err(|error| {
				CommandError::ExecutionError(format!(
					"command `{command}` requires settings `{}`: {error}",
					requirement.name
				))
			})?;
			context.values.insert(key, value);
		}
		for requirement in requirements
			.iter()
			.filter(|req| req.kind == CapabilityKind::Service)
		{
			let key = requirement.key();
			if context.values.contains_key(&key) {
				continue;
			}
			let value = provider.prepare_service(requirement, &context).await?;
			if value.as_ref().type_id() != requirement.type_id {
				return Err(CommandError::ExecutionError(format!(
					"provider supplied the wrong type for capability `{}` in command `{command}`",
					requirement.name
				)));
			}
			context.values.insert(key, value);
		}
		Ok(context)
	}
}

/// Application-owned settings and service bootstrap for the new entry point.
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
	/// Fully composed settings type used for strict runtime commands.
	type Settings: ComposedSettings
		+ HasCommonSettings
		+ HasSettings<MigrationSettings>
		+ Send
		+ Sync
		+ 'static;

	/// Build raw, syntax-checked settings with deferred interpolation.
	fn scoped_settings(&self) -> Result<ScopedSettings, BuildError>;

	/// Build strict full settings for legacy and runtime commands.
	fn full_settings(&self) -> Result<PendingSettings<Self::Settings>, BuildError>;

	/// Prepare a declared application service after settings validation.
	/// The returned value owns all resources through RAII for this invocation.
	async fn prepare_service(
		&self,
		requirement: &CapabilityRequirement,
		_context: &CapabilityContext,
	) -> CommandResult<PreparedValue> {
		Err(CommandError::ExecutionError(format!(
			"no provider for service `{}`{}; register a service provider",
			requirement.name(),
			requirement
				.alias()
				.map(|alias| format!(" (alias `{alias}`)"))
				.unwrap_or_default()
		)))
	}
}

/// Opt-in custom command with pure CLI metadata and explicit requirements.
#[async_trait]
pub trait CapabilityCommand: Send + Sync {
	/// A clap command definition used by the shared management CLI.
	fn cli(&self) -> Command;

	/// Requirements selected from successfully parsed arguments.
	fn requirements(&self, matches: &ArgMatches) -> Vec<CapabilityRequirement>;

	/// Execute using already prepared capabilities.
	async fn execute(&self, matches: &ArgMatches, context: &CapabilityContext)
	-> CommandResult<()>;
}

/// Adapter for applications whose scoped source is a settings builder.
pub fn build_scoped_settings(builder: SettingsBuilder) -> Result<ScopedSettings, BuildError> {
	builder.build_scoped()
}
