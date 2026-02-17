//! Migration dependency types
//!
//! This module provides types for managing migration dependencies, including:
//! - Required dependencies (standard)
//! - Swappable dependencies (e.g., AUTH_USER_MODEL pattern)
//! - Optional dependencies (conditional based on app installation or settings)
//!
//! # Example
//!
//! ```rust
//! use reinhardt_db::migrations::dependency::{
//!     MigrationDependency, SwappableDependency, OptionalDependency, DependencyCondition
//! };
//!
//! // Required dependency
//! let required = MigrationDependency::Required {
//!     app_label: "auth".to_string(),
//!     migration_name: "0001_initial".to_string(),
//! };
//!
//! // Swappable dependency (depends on configured User model)
//! let swappable = MigrationDependency::Swappable(SwappableDependency {
//!     setting_key: "AUTH_USER_MODEL".to_string(),
//!     default_app: "auth".to_string(),
//!     default_model: "User".to_string(),
//!     migration_name: "0001_initial".to_string(),
//! });
//!
//! // Optional dependency (only if GIS app is installed)
//! let optional = MigrationDependency::Optional(OptionalDependency {
//!     app_label: "gis_extension".to_string(),
//!     migration_name: "0001_enable_postgis".to_string(),
//!     condition: DependencyCondition::AppInstalled("gis_extension".to_string()),
//! });
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A dependency that resolves to different apps based on settings.
///
/// This is used for Django's swappable model pattern (e.g., AUTH_USER_MODEL).
/// When a migration depends on a model that can be swapped out (like the User model),
/// this dependency type allows the migration system to resolve to the actual
/// configured model at runtime.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::dependency::SwappableDependency;
///
/// let dep = SwappableDependency {
///     setting_key: "AUTH_USER_MODEL".to_string(),
///     default_app: "auth".to_string(),
///     default_model: "User".to_string(),
///     migration_name: "0001_initial".to_string(),
/// };
///
/// // In Django, AUTH_USER_MODEL might be set to "myapp.CustomUser"
/// // The migration system would resolve this to ("myapp", "0001_initial")
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwappableDependency {
	/// Setting key to look up (e.g., "AUTH_USER_MODEL")
	pub setting_key: String,

	/// Default app label if setting is not configured
	pub default_app: String,

	/// Default model name if setting is not configured
	pub default_model: String,

	/// Migration name to depend on (typically "0001_initial" or "__first__")
	pub migration_name: String,
}

impl SwappableDependency {
	/// Create a new swappable dependency
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::SwappableDependency;
	///
	/// let dep = SwappableDependency::new(
	///     "AUTH_USER_MODEL",
	///     "auth",
	///     "User",
	///     "0001_initial",
	/// );
	/// assert_eq!(dep.setting_key, "AUTH_USER_MODEL");
	/// ```
	pub fn new(
		setting_key: impl Into<String>,
		default_app: impl Into<String>,
		default_model: impl Into<String>,
		migration_name: impl Into<String>,
	) -> Self {
		Self {
			setting_key: setting_key.into(),
			default_app: default_app.into(),
			default_model: default_model.into(),
			migration_name: migration_name.into(),
		}
	}

	/// Resolve the swappable dependency to an actual app label.
	///
	/// This method looks up the setting value and extracts the app label.
	/// If the setting is not configured, returns the default app.
	///
	/// # Arguments
	///
	/// * `setting_value` - The value from settings (e.g., "myapp.CustomUser")
	///
	/// # Returns
	///
	/// The app label to use for the dependency.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::SwappableDependency;
	///
	/// let dep = SwappableDependency::new("AUTH_USER_MODEL", "auth", "User", "0001_initial");
	///
	/// // With custom setting
	/// assert_eq!(dep.resolve_app_label(Some("myapp.CustomUser")), "myapp");
	///
	/// // Without setting (uses default)
	/// assert_eq!(dep.resolve_app_label(None), "auth");
	/// ```
	pub fn resolve_app_label(&self, setting_value: Option<&str>) -> String {
		match setting_value {
			Some(value) => {
				// Parse "app_label.ModelName" format
				if let Some((app, _model)) = value.split_once('.') {
					app.to_string()
				} else {
					// If no dot, assume it's just the app label
					value.to_string()
				}
			}
			None => self.default_app.clone(),
		}
	}

	/// Resolve to a dependency tuple (app_label, migration_name).
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::SwappableDependency;
	///
	/// let dep = SwappableDependency::new("AUTH_USER_MODEL", "auth", "User", "0001_initial");
	/// let (app, migration) = dep.resolve(Some("custom_auth.MyUser"));
	///
	/// assert_eq!(app, "custom_auth");
	/// assert_eq!(migration, "0001_initial");
	/// ```
	pub fn resolve(&self, setting_value: Option<&str>) -> (String, String) {
		(
			self.resolve_app_label(setting_value),
			self.migration_name.clone(),
		)
	}
}

/// Conditions for optional dependencies.
///
/// Optional dependencies are only enforced when their condition is met.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyCondition {
	/// Dependency is required only if the specified app is installed.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::DependencyCondition;
	///
	/// // Depend on GIS extension only if it's installed
	/// let condition = DependencyCondition::AppInstalled("gis_extension".to_string());
	/// ```
	AppInstalled(String),

	/// Dependency is required only if the specified setting is enabled (truthy).
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::DependencyCondition;
	///
	/// // Depend on audit tables only if ENABLE_AUDIT_LOGGING is true
	/// let condition = DependencyCondition::SettingEnabled("ENABLE_AUDIT_LOGGING".to_string());
	/// ```
	SettingEnabled(String),

	/// Dependency is required only if the specified feature flag is enabled.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::DependencyCondition;
	///
	/// // Depend on feature-specific migrations
	/// let condition = DependencyCondition::FeatureEnabled("advanced_search".to_string());
	/// ```
	FeatureEnabled(String),
}

impl DependencyCondition {
	/// Check if the condition is satisfied.
	///
	/// # Arguments
	///
	/// * `installed_apps` - Set of installed app labels
	/// * `settings` - Function to look up setting values
	/// * `features` - Set of enabled feature flags
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::DependencyCondition;
	/// use std::collections::HashSet;
	///
	/// let mut apps = HashSet::new();
	/// apps.insert("gis_extension".to_string());
	///
	/// let condition = DependencyCondition::AppInstalled("gis_extension".to_string());
	///
	/// assert!(condition.is_satisfied(
	///     &apps,
	///     &|_| None,
	///     &HashSet::new(),
	/// ));
	/// ```
	pub fn is_satisfied<F>(
		&self,
		installed_apps: &HashSet<String>,
		settings_lookup: &F,
		features: &HashSet<String>,
	) -> bool
	where
		F: Fn(&str) -> Option<String>,
	{
		match self {
			DependencyCondition::AppInstalled(app) => installed_apps.contains(app),
			DependencyCondition::SettingEnabled(key) => {
				if let Some(value) = settings_lookup(key) {
					is_truthy(&value)
				} else {
					false
				}
			}
			DependencyCondition::FeatureEnabled(feature) => features.contains(feature),
		}
	}
}

/// An optional dependency that is only enforced when a condition is met.
///
/// This is useful for migrations that depend on optional features or apps.
/// For example, a migration might depend on PostGIS extensions only if the
/// GIS app is installed.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::dependency::{OptionalDependency, DependencyCondition};
///
/// let dep = OptionalDependency {
///     app_label: "gis_extension".to_string(),
///     migration_name: "0001_enable_postgis".to_string(),
///     condition: DependencyCondition::AppInstalled("gis_extension".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalDependency {
	/// Target app label
	pub app_label: String,

	/// Target migration name
	pub migration_name: String,

	/// Condition that must be met for this dependency to be enforced
	pub condition: DependencyCondition,
}

impl OptionalDependency {
	/// Create a new optional dependency
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::{OptionalDependency, DependencyCondition};
	///
	/// let dep = OptionalDependency::new(
	///     "gis_extension",
	///     "0001_enable_postgis",
	///     DependencyCondition::AppInstalled("gis_extension".to_string()),
	/// );
	/// ```
	pub fn new(
		app_label: impl Into<String>,
		migration_name: impl Into<String>,
		condition: DependencyCondition,
	) -> Self {
		Self {
			app_label: app_label.into(),
			migration_name: migration_name.into(),
			condition,
		}
	}

	/// Check if this optional dependency should be enforced.
	///
	/// # Arguments
	///
	/// * `installed_apps` - Set of installed app labels
	/// * `settings_lookup` - Function to look up setting values
	/// * `features` - Set of enabled feature flags
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::{OptionalDependency, DependencyCondition};
	/// use std::collections::HashSet;
	///
	/// let dep = OptionalDependency::new(
	///     "gis",
	///     "0001_initial",
	///     DependencyCondition::AppInstalled("gis".to_string()),
	/// );
	///
	/// let mut apps = HashSet::new();
	/// // GIS not installed
	/// assert!(!dep.should_enforce(&apps, &|_| None, &HashSet::new()));
	///
	/// // GIS installed
	/// apps.insert("gis".to_string());
	/// assert!(dep.should_enforce(&apps, &|_| None, &HashSet::new()));
	/// ```
	pub fn should_enforce<F>(
		&self,
		installed_apps: &HashSet<String>,
		settings_lookup: &F,
		features: &HashSet<String>,
	) -> bool
	where
		F: Fn(&str) -> Option<String>,
	{
		self.condition
			.is_satisfied(installed_apps, settings_lookup, features)
	}

	/// Convert to a dependency tuple if the condition is satisfied.
	///
	/// Returns `Some((app_label, migration_name))` if the condition is met,
	/// `None` otherwise.
	pub fn to_dependency_if_satisfied<F>(
		&self,
		installed_apps: &HashSet<String>,
		settings_lookup: &F,
		features: &HashSet<String>,
	) -> Option<(String, String)>
	where
		F: Fn(&str) -> Option<String>,
	{
		if self.should_enforce(installed_apps, settings_lookup, features) {
			Some((self.app_label.clone(), self.migration_name.clone()))
		} else {
			None
		}
	}
}

/// Unified migration dependency type.
///
/// This enum represents all types of dependencies a migration can have:
/// - Required: Always enforced
/// - Swappable: Resolves to different apps based on settings
/// - Optional: Only enforced when a condition is met
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::dependency::{
///     MigrationDependency, SwappableDependency, OptionalDependency, DependencyCondition
/// };
///
/// let deps = vec![
///     MigrationDependency::Required {
///         app_label: "auth".to_string(),
///         migration_name: "0001_initial".to_string(),
///     },
///     MigrationDependency::Swappable(SwappableDependency::new(
///         "AUTH_USER_MODEL",
///         "auth",
///         "User",
///         "0001_initial",
///     )),
///     MigrationDependency::Optional(OptionalDependency::new(
///         "gis",
///         "0001_initial",
///         DependencyCondition::AppInstalled("gis".to_string()),
///     )),
/// ];
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationDependency {
	/// A standard required dependency.
	Required {
		/// Target app label
		app_label: String,
		/// Target migration name
		migration_name: String,
	},

	/// A swappable dependency that resolves based on settings.
	Swappable(SwappableDependency),

	/// An optional dependency that is only enforced when a condition is met.
	Optional(OptionalDependency),
}

impl MigrationDependency {
	/// Create a required dependency.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::MigrationDependency;
	///
	/// let dep = MigrationDependency::required("auth", "0001_initial");
	/// ```
	pub fn required(app_label: impl Into<String>, migration_name: impl Into<String>) -> Self {
		Self::Required {
			app_label: app_label.into(),
			migration_name: migration_name.into(),
		}
	}

	/// Create a swappable dependency.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::MigrationDependency;
	///
	/// let dep = MigrationDependency::swappable(
	///     "AUTH_USER_MODEL",
	///     "auth",
	///     "User",
	///     "0001_initial",
	/// );
	/// ```
	pub fn swappable(
		setting_key: impl Into<String>,
		default_app: impl Into<String>,
		default_model: impl Into<String>,
		migration_name: impl Into<String>,
	) -> Self {
		Self::Swappable(SwappableDependency::new(
			setting_key,
			default_app,
			default_model,
			migration_name,
		))
	}

	/// Create an optional dependency with AppInstalled condition.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::MigrationDependency;
	///
	/// let dep = MigrationDependency::optional_app(
	///     "gis",
	///     "0001_initial",
	///     "gis",
	/// );
	/// ```
	pub fn optional_app(
		app_label: impl Into<String>,
		migration_name: impl Into<String>,
		required_app: impl Into<String>,
	) -> Self {
		Self::Optional(OptionalDependency::new(
			app_label,
			migration_name,
			DependencyCondition::AppInstalled(required_app.into()),
		))
	}

	/// Create an optional dependency with SettingEnabled condition.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::MigrationDependency;
	///
	/// let dep = MigrationDependency::optional_setting(
	///     "audit",
	///     "0001_initial",
	///     "ENABLE_AUDIT",
	/// );
	/// ```
	pub fn optional_setting(
		app_label: impl Into<String>,
		migration_name: impl Into<String>,
		setting_key: impl Into<String>,
	) -> Self {
		Self::Optional(OptionalDependency::new(
			app_label,
			migration_name,
			DependencyCondition::SettingEnabled(setting_key.into()),
		))
	}

	/// Create an optional dependency with FeatureEnabled condition.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::MigrationDependency;
	///
	/// let dep = MigrationDependency::optional_feature(
	///     "search",
	///     "0001_initial",
	///     "advanced_search",
	/// );
	/// ```
	pub fn optional_feature(
		app_label: impl Into<String>,
		migration_name: impl Into<String>,
		feature: impl Into<String>,
	) -> Self {
		Self::Optional(OptionalDependency::new(
			app_label,
			migration_name,
			DependencyCondition::FeatureEnabled(feature.into()),
		))
	}
}

/// Context for resolving dependencies.
///
/// This struct holds all the information needed to resolve swappable and
/// optional dependencies to their actual targets.
#[derive(Debug, Clone, Default)]
pub struct DependencyResolutionContext {
	/// Set of installed app labels
	pub installed_apps: HashSet<String>,

	/// Map of setting key to value for swappable dependencies
	pub swappable_settings: std::collections::HashMap<String, String>,

	/// Set of enabled feature flags
	pub features: HashSet<String>,
}

impl DependencyResolutionContext {
	/// Create a new empty context.
	pub fn new() -> Self {
		Self::default()
	}

	/// Add an installed app.
	pub fn with_app(mut self, app: impl Into<String>) -> Self {
		self.installed_apps.insert(app.into());
		self
	}

	/// Add multiple installed apps.
	pub fn with_apps(mut self, apps: impl IntoIterator<Item = impl Into<String>>) -> Self {
		for app in apps {
			self.installed_apps.insert(app.into());
		}
		self
	}

	/// Add a swappable setting.
	pub fn with_setting(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.swappable_settings.insert(key.into(), value.into());
		self
	}

	/// Add a feature flag.
	pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
		self.features.insert(feature.into());
		self
	}

	/// Look up a setting value.
	pub fn get_setting(&self, key: &str) -> Option<&String> {
		self.swappable_settings.get(key)
	}

	/// Check if an app is installed.
	pub fn is_app_installed(&self, app: &str) -> bool {
		self.installed_apps.contains(app)
	}

	/// Check if a feature is enabled.
	pub fn is_feature_enabled(&self, feature: &str) -> bool {
		self.features.contains(feature)
	}
}

/// Resolver for migration dependencies.
///
/// This struct handles the resolution of all dependency types to their
/// actual (app_label, migration_name) tuples.
pub struct DependencyResolver<'a> {
	context: &'a DependencyResolutionContext,
}

impl<'a> DependencyResolver<'a> {
	/// Create a new resolver with the given context.
	pub fn new(context: &'a DependencyResolutionContext) -> Self {
		Self { context }
	}

	/// Resolve a single dependency to its actual target.
	///
	/// Returns `Some((app_label, migration_name))` if the dependency should be
	/// enforced, `None` if it's an optional dependency whose condition is not met.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::{
	///     MigrationDependency, DependencyResolver, DependencyResolutionContext
	/// };
	///
	/// let context = DependencyResolutionContext::new()
	///     .with_setting("AUTH_USER_MODEL", "custom_auth.CustomUser");
	///
	/// let resolver = DependencyResolver::new(&context);
	///
	/// let dep = MigrationDependency::swappable(
	///     "AUTH_USER_MODEL",
	///     "auth",
	///     "User",
	///     "0001_initial",
	/// );
	///
	/// let resolved = resolver.resolve(&dep);
	/// assert_eq!(resolved, Some(("custom_auth".to_string(), "0001_initial".to_string())));
	/// ```
	pub fn resolve(&self, dependency: &MigrationDependency) -> Option<(String, String)> {
		match dependency {
			MigrationDependency::Required {
				app_label,
				migration_name,
			} => Some((app_label.clone(), migration_name.clone())),

			MigrationDependency::Swappable(swappable) => {
				let setting_value = self
					.context
					.get_setting(&swappable.setting_key)
					.map(|s| s.as_str());
				Some(swappable.resolve(setting_value))
			}

			MigrationDependency::Optional(optional) => {
				let settings_lookup = |key: &str| self.context.get_setting(key).cloned();

				optional.to_dependency_if_satisfied(
					&self.context.installed_apps,
					&settings_lookup,
					&self.context.features,
				)
			}
		}
	}

	/// Resolve multiple dependencies, filtering out unsatisfied optional dependencies.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::dependency::{
	///     MigrationDependency, DependencyResolver, DependencyResolutionContext
	/// };
	///
	/// let context = DependencyResolutionContext::new()
	///     .with_app("auth");
	///
	/// let resolver = DependencyResolver::new(&context);
	///
	/// let deps = vec![
	///     MigrationDependency::required("auth", "0001_initial"),
	///     MigrationDependency::optional_app("gis", "0001_initial", "gis"),
	/// ];
	///
	/// let resolved = resolver.resolve_all(&deps);
	/// // Only the required dependency is resolved (gis not installed)
	/// assert_eq!(resolved.len(), 1);
	/// assert_eq!(resolved[0], ("auth".to_string(), "0001_initial".to_string()));
	/// ```
	pub fn resolve_all(&self, dependencies: &[MigrationDependency]) -> Vec<(String, String)> {
		dependencies
			.iter()
			.filter_map(|dep| self.resolve(dep))
			.collect()
	}
}

/// Check if a string value is "truthy" (non-empty and not "false"/"0"/"no").
fn is_truthy(value: &str) -> bool {
	let lower = value.to_lowercase();
	!value.is_empty() && lower != "false" && lower != "0" && lower != "no" && lower != "off"
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_swappable_dependency_resolve_with_setting() {
		let dep = SwappableDependency::new("AUTH_USER_MODEL", "auth", "User", "0001_initial");

		let (app, migration) = dep.resolve(Some("custom_auth.CustomUser"));
		assert_eq!(app, "custom_auth");
		assert_eq!(migration, "0001_initial");
	}

	#[rstest]
	fn test_swappable_dependency_resolve_without_setting() {
		let dep = SwappableDependency::new("AUTH_USER_MODEL", "auth", "User", "0001_initial");

		let (app, migration) = dep.resolve(None);
		assert_eq!(app, "auth");
		assert_eq!(migration, "0001_initial");
	}

	#[rstest]
	fn test_optional_dependency_app_installed() {
		let dep = OptionalDependency::new(
			"gis",
			"0001_initial",
			DependencyCondition::AppInstalled("gis".to_string()),
		);

		let mut apps = HashSet::new();

		// Not installed
		assert!(!dep.should_enforce(&apps, &|_| None, &HashSet::new()));

		// Installed
		apps.insert("gis".to_string());
		assert!(dep.should_enforce(&apps, &|_| None, &HashSet::new()));
	}

	#[rstest]
	fn test_optional_dependency_setting_enabled() {
		let dep = OptionalDependency::new(
			"audit",
			"0001_initial",
			DependencyCondition::SettingEnabled("ENABLE_AUDIT".to_string()),
		);

		let apps = HashSet::new();

		// Setting not present
		assert!(!dep.should_enforce(&apps, &|_| None, &HashSet::new()));

		// Setting is false
		assert!(!dep.should_enforce(
			&apps,
			&|key| {
				if key == "ENABLE_AUDIT" {
					Some("false".to_string())
				} else {
					None
				}
			},
			&HashSet::new()
		));

		// Setting is true
		assert!(dep.should_enforce(
			&apps,
			&|key| {
				if key == "ENABLE_AUDIT" {
					Some("true".to_string())
				} else {
					None
				}
			},
			&HashSet::new()
		));
	}

	#[rstest]
	fn test_dependency_resolver() {
		let context = DependencyResolutionContext::new()
			.with_app("auth")
			.with_app("users")
			.with_setting("AUTH_USER_MODEL", "custom_auth.CustomUser");

		let resolver = DependencyResolver::new(&context);

		// Required dependency
		let required = MigrationDependency::required("auth", "0001_initial");
		assert_eq!(
			resolver.resolve(&required),
			Some(("auth".to_string(), "0001_initial".to_string()))
		);

		// Swappable dependency
		let swappable =
			MigrationDependency::swappable("AUTH_USER_MODEL", "auth", "User", "0001_initial");
		assert_eq!(
			resolver.resolve(&swappable),
			Some(("custom_auth".to_string(), "0001_initial".to_string()))
		);

		// Optional dependency (satisfied)
		let optional_satisfied = MigrationDependency::optional_app("auth", "0001_initial", "auth");
		assert_eq!(
			resolver.resolve(&optional_satisfied),
			Some(("auth".to_string(), "0001_initial".to_string()))
		);

		// Optional dependency (not satisfied)
		let optional_not_satisfied =
			MigrationDependency::optional_app("gis", "0001_initial", "gis");
		assert_eq!(resolver.resolve(&optional_not_satisfied), None);
	}

	#[rstest]
	fn test_resolve_all_filters_unsatisfied() {
		let context = DependencyResolutionContext::new().with_app("auth");

		let resolver = DependencyResolver::new(&context);

		let deps = vec![
			MigrationDependency::required("auth", "0001_initial"),
			MigrationDependency::optional_app("gis", "0001_initial", "gis"),
			MigrationDependency::required("users", "0001_initial"),
		];

		let resolved = resolver.resolve_all(&deps);
		assert_eq!(resolved.len(), 2);
		assert!(resolved.contains(&("auth".to_string(), "0001_initial".to_string())));
		assert!(resolved.contains(&("users".to_string(), "0001_initial".to_string())));
	}

	#[rstest]
	fn test_is_truthy() {
		assert!(is_truthy("true"));
		assert!(is_truthy("True"));
		assert!(is_truthy("TRUE"));
		assert!(is_truthy("1"));
		assert!(is_truthy("yes"));
		assert!(is_truthy("on"));
		assert!(is_truthy("enabled"));

		assert!(!is_truthy("false"));
		assert!(!is_truthy("False"));
		assert!(!is_truthy("FALSE"));
		assert!(!is_truthy("0"));
		assert!(!is_truthy("no"));
		assert!(!is_truthy("off"));
		assert!(!is_truthy(""));
	}
}
