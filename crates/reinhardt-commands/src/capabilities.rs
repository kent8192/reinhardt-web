//! Opt-in command capability preparation and invocation-owned resources.

use crate::local_infra::DatabaseInfraInput;
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
use std::path::PathBuf;
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
		if crate::database_selector::alias_looks_sensitive(alias) {
			return Err(BuildError::Deserialization(
				"database alias must be a name, not a connection URL".to_owned(),
			));
		}
		let config = settings
			.first_present_path::<DatabaseConfig>(&[
				&["core", "databases", alias],
				&["databases", alias],
			])?
			.ok_or_else(|| {
				BuildError::Deserialization(format!(
					"missing required settings path `core.databases.{alias}`"
				))
			})?;
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

/// Core migration inputs that do not require a runtime secret or database.
pub struct CoreMigrationMetadata {
	/// Project base directory used for the default migration-file location.
	pub base_dir: PathBuf,
	/// Application names used for dependency resolution.
	pub installed_apps: Vec<String>,
	/// Conditional migration dependency features.
	pub migration_features: Vec<String>,
	/// Swappable migration dependency settings.
	pub migration_swappable_settings: HashMap<String, String>,
}

impl SettingsView for CoreMigrationMetadata {
	const NAME: &'static str = "core migration metadata";

	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError> {
		if alias.is_some() {
			return Err(BuildError::Deserialization(
				"core migration metadata does not accept an alias".to_owned(),
			));
		}
		Ok(Self {
			base_dir: settings
				.first_present_path(&[&["core", "base_dir"], &["base_dir"]])?
				.unwrap_or_else(|| PathBuf::from(".")),
			installed_apps: settings
				.first_present_path(&[&["core", "installed_apps"], &["installed_apps"]])?
				.unwrap_or_default(),
			migration_features: settings
				.first_present_path(&[&["core", "migration_features"], &["migration_features"]])?
				.unwrap_or_default(),
			migration_swappable_settings: settings
				.first_present_path(&[
					&["core", "migration_swappable_settings"],
					&["migration_swappable_settings"],
				])?
				.unwrap_or_default(),
		})
	}
}

/// Local infrastructure configuration selected without full application settings.
pub struct LocalInfrastructureSettings {
	/// PostgreSQL input, if the selected local database uses PostgreSQL.
	pub database: Option<DatabaseInfraInput>,
}

/// Typed inputs for the selected system diagnostics.
pub struct CheckInputs {
	pub(crate) database_url: Option<String>,
	pub(crate) static_root_configured: bool,
	pub(crate) secret_key_length: Option<usize>,
	pub(crate) debug: Option<bool>,
	pub(crate) allowed_hosts_configured: bool,
	pub(crate) ssl_redirect: bool,
}

impl SettingsView for CheckInputs {
	const NAME: &'static str = "system check inputs";

	fn resolve(settings: &ScopedSettings, mode: Option<&str>) -> Result<Self, BuildError> {
		let deploy = match mode {
			None => false,
			Some("deploy") => true,
			Some(_) => return Err(BuildError::Deserialization("unknown check mode".to_owned())),
		};
		let database_url = match std::env::var("DATABASE_URL") {
			Ok(url) => Some(url),
			Err(_)
				if settings.has_path(&["core", "databases", "default"])
					|| settings.has_path(&["databases", "default"]) =>
			{
				Some(SelectedDatabase::resolve(settings, Some("default"))?.url())
			}
			Err(_) => None,
		};
		let static_root: Option<String> = settings.first_present_path(&[
			&["static_files", "root"],
			&["static", "root"],
			&["static_root"],
		])?;
		let secret_key_length = if deploy {
			settings
				.first_present_path::<String>(&[&["core", "secret_key"], &["secret_key"]])?
				.or_else(|| std::env::var("SECRET_KEY").ok())
				.map(|value| value.len())
		} else {
			None
		};
		let debug = settings
			.first_present_path(&[&["core", "debug"], &["debug"]])?
			.or_else(|| std::env::var("DEBUG").ok().map(|value| value == "true"))
			.or(deploy.then_some(true));
		let allowed_hosts_configured = if deploy {
			settings
				.first_present_path::<Vec<String>>(&[
					&["core", "allowed_hosts"],
					&["allowed_hosts"],
				])?
				.is_some_and(|hosts| !hosts.is_empty())
				|| std::env::var_os("ALLOWED_HOSTS").is_some()
		} else {
			false
		};
		let ssl_redirect = if deploy {
			settings
				.first_present_path(&[
					&["core", "security", "secure_ssl_redirect"],
					&["security", "secure_ssl_redirect"],
				])?
				.unwrap_or_else(|| {
					std::env::var("SECURE_SSL_REDIRECT").is_ok_and(|value| value == "true")
				})
		} else {
			false
		};
		Ok(Self {
			database_url,
			static_root_configured: static_root.is_some_and(|root| !root.is_empty())
				|| std::env::var_os("STATIC_ROOT").is_some(),
			secret_key_length,
			debug,
			allowed_hosts_configured,
			ssl_redirect,
		})
	}
}

impl SettingsView for LocalInfrastructureSettings {
	const NAME: &'static str = "local infrastructure settings";

	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError> {
		if alias.is_some() {
			return Err(BuildError::Deserialization(
				"local infrastructure settings do not accept an alias".to_owned(),
			));
		}
		let paths: &[&[&str]] = &[&["core", "databases", "default"], &["databases", "default"]];
		let configured = paths.iter().any(|path| settings.has_path(path));
		let engine: Option<String> = settings.first_present_path(&[
			&["core", "databases", "default", "engine"],
			&["databases", "default", "engine"],
		])?;
		if configured && engine.is_none() {
			return Err(BuildError::Deserialization(
				"selected local database requires an engine".to_owned(),
			));
		}
		let database = if engine.is_some_and(|engine| engine.contains("postgres")) {
			let config: DatabaseConfig = settings.first_present_path(paths)?.ok_or_else(|| {
				BuildError::Deserialization(
					"missing selected local PostgreSQL database settings".to_owned(),
				)
			})?;
			if config.name.is_empty() {
				return Err(BuildError::Deserialization(
					"local PostgreSQL database requires a non-empty name".to_owned(),
				));
			}
			Some(DatabaseInfraInput {
				engine: config.engine,
				host: config.host.unwrap_or_else(|| "127.0.0.1".to_owned()),
				port: config.port.unwrap_or(5432),
				name: config.name,
				user: config.user.unwrap_or_else(|| "postgres".to_owned()),
				password: config
					.password
					.map(|password| password.expose_secret().to_owned()),
			})
		} else {
			None
		};
		Ok(Self { database })
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
					.map(|value| format!(" (alias `{}`)", safe_alias(value)))
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
				.map(|alias| format!(" (alias `{}`)", safe_alias(alias)))
				.unwrap_or_default()
		)))
	}
}

fn safe_alias(alias: &str) -> &str {
	if crate::database_selector::alias_looks_sensitive(alias) {
		"[REDACTED]"
	} else {
		alias
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
