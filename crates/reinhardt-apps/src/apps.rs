//! # Application Registry
//!
//! Django-inspired application configuration and registry system.
//! This module provides the infrastructure for managing Django-style apps
//! in a Reinhardt project.
//!
//! This module provides both string-based (runtime) and type-safe (compile-time)
//! application registry mechanisms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use thiserror::Error;

/// Errors that can occur when working with the application registry
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Application not found: {0}")]
    NotFound(String),

    #[error("Application already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Invalid application label: {0}")]
    InvalidLabel(String),

    #[error("Duplicate application label: {0}")]
    DuplicateLabel(String),

    #[error("Duplicate application name: {0}")]
    DuplicateName(String),

    #[error("Application registry not ready")]
    NotReady,

    #[error("Application configuration error: {0}")]
    ConfigError(String),
}

pub type AppResult<T> = Result<T, AppError>;

/// Configuration for a single application
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// The full Python-style name of the application (e.g., "myapp" or "myproject.apps.MyAppConfig")
    pub name: String,

    /// The short label for the application (e.g., "myapp")
    pub label: String,

    /// Human-readable name for the application
    pub verbose_name: Option<String>,

    /// Filesystem path to the application
    pub path: Option<String>,

    /// Default auto field type for models in this app
    pub default_auto_field: Option<String>,

    /// Whether the app has been populated with models
    pub models_ready: bool,
}

impl AppConfig {
    /// Create a new AppConfig with required fields
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            verbose_name: None,
            path: None,
            default_auto_field: None,
            models_ready: false,
        }
    }

    /// Set the verbose name for the application
    pub fn with_verbose_name(mut self, verbose_name: impl Into<String>) -> Self {
        self.verbose_name = Some(verbose_name.into());
        self
    }

    /// Set the path for the application
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the default auto field for the application
    pub fn with_default_auto_field(mut self, field: impl Into<String>) -> Self {
        self.default_auto_field = Some(field.into());
        self
    }

    /// Validate the application label
    pub fn validate_label(&self) -> AppResult<()> {
        if self.label.is_empty() {
            return Err(AppError::InvalidLabel("Label cannot be empty".to_string()));
        }

        // Check if label is a valid Rust identifier
        if !self
            .label
            .chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
        {
            return Err(AppError::InvalidLabel(format!(
                "Label '{}' must start with a letter or underscore",
                self.label
            )));
        }

        if !self.label.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AppError::InvalidLabel(format!(
                "Label '{}' must contain only alphanumeric characters and underscores",
                self.label
            )));
        }

        Ok(())
    }
}

/// Main application registry
///
/// This is the central registry for all installed applications in a Reinhardt project.
/// It manages application configuration, initialization order, and provides
/// methods to query installed applications.
#[derive(Clone)]
pub struct Apps {
    /// List of installed application identifiers
    installed_apps: Vec<String>,

    /// Map of application labels to their configurations
    app_configs: Arc<Mutex<HashMap<String, AppConfig>>>,

    /// Map of application names to their labels
    app_names: Arc<Mutex<HashMap<String, String>>>,

    /// Whether the registry has been populated
    ready: Arc<Mutex<bool>>,

    /// Whether app configs have been populated
    apps_ready: Arc<Mutex<bool>>,

    /// Whether models have been populated
    models_ready: Arc<Mutex<bool>>,
}

impl Apps {
    /// Create a new application registry
    pub fn new(installed_apps: Vec<String>) -> Self {
        Self {
            installed_apps,
            app_configs: Arc::new(Mutex::new(HashMap::new())),
            app_names: Arc::new(Mutex::new(HashMap::new())),
            ready: Arc::new(Mutex::new(false)),
            apps_ready: Arc::new(Mutex::new(false)),
            models_ready: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if the registry is fully ready
    pub fn is_ready(&self) -> bool {
        *self.ready.lock().unwrap()
    }

    /// Check if app configurations are ready
    pub fn is_apps_ready(&self) -> bool {
        *self.apps_ready.lock().unwrap()
    }

    /// Check if models are ready
    pub fn is_models_ready(&self) -> bool {
        *self.models_ready.lock().unwrap()
    }

    /// Register an application configuration
    pub fn register(&self, config: AppConfig) -> AppResult<()> {
        // Validate the configuration
        config.validate_label()?;

        let mut configs = self.app_configs.lock().unwrap();
        let mut names = self.app_names.lock().unwrap();

        // Check for duplicate label
        if configs.contains_key(&config.label) {
            return Err(AppError::DuplicateLabel(config.label.clone()));
        }

        // Check for duplicate name
        if names.contains_key(&config.name) {
            return Err(AppError::DuplicateName(config.name.clone()));
        }

        // Store the configuration
        names.insert(config.name.clone(), config.label.clone());
        configs.insert(config.label.clone(), config);

        Ok(())
    }

    /// Get an application configuration by label
    pub fn get_app_config(&self, label: &str) -> AppResult<AppConfig> {
        self.app_configs
            .lock()
            .unwrap()
            .get(label)
            .cloned()
            .ok_or_else(|| AppError::NotFound(label.to_string()))
    }

    /// Get all registered application configurations
    pub fn get_app_configs(&self) -> Vec<AppConfig> {
        self.app_configs.lock().unwrap().values().cloned().collect()
    }

    /// Check if an application is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed_apps.contains(&name.to_string())
            || self.app_names.lock().unwrap().contains_key(name)
            || self.app_configs.lock().unwrap().contains_key(name)
    }

    /// Populate the registry with application configurations
    ///
    /// This method initializes all registered applications by:
    /// 1. Creating AppConfig instances for each installed app
    /// 2. Calling the ready() method on each AppConfig
    /// 3. Loading model definitions from the global registry
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::Apps;
    ///
    /// let apps = Apps::new(vec!["myapp".to_string()]);
    /// apps.populate().expect("Failed to populate apps");
    /// ```
    pub fn populate(&self) -> AppResult<()> {
        // Mark as apps_ready
        *self.apps_ready.lock().unwrap() = true;

        // 1. Import and instantiate AppConfig for each installed app
        for app_name in &self.installed_apps {
            let app_config = AppConfig::new(app_name.clone(), app_name.clone());

            // Store in registries
            self.app_configs
                .lock()
                .unwrap()
                .insert(app_config.label.clone(), app_config.clone());
            self.app_names
                .lock()
                .unwrap()
                .insert(app_name.clone(), app_config.label.clone());
        }

        // 2. Call ready() method on each AppConfig (currently no-op)
        // In the future, this would call custom ready() hooks for each app

        // 3. Load model definitions from global ModelRegistry
        // The models are already registered via #[derive(Model)] macro
        // which automatically registers them at construction time

        // 4. Build reverse relations between models
        // This would require analyzing foreign key relationships
        // For now, this is deferred until ORM relationship system is fully implemented

        // Mark as models_ready
        *self.models_ready.lock().unwrap() = true;
        *self.ready.lock().unwrap() = true;

        Ok(())
    }

    /// Clear all cached data (for testing)
    pub fn clear_cache(&self) {
        self.app_configs.lock().unwrap().clear();
        self.app_names.lock().unwrap().clear();
        *self.ready.lock().unwrap() = false;
        *self.apps_ready.lock().unwrap() = false;
        *self.models_ready.lock().unwrap() = false;
    }
}

/// Global application registry singleton
static GLOBAL_APPS: OnceLock<Apps> = OnceLock::new();

/// Get the global application registry
pub fn get_apps() -> &'static Apps {
    GLOBAL_APPS.get_or_init(|| Apps::new(vec![]))
}

/// Initialize the global application registry with a list of installed apps
pub fn init_apps(installed_apps: Vec<String>) -> AppResult<()> {
    if GLOBAL_APPS.get().is_some() {
        return Err(AppError::AlreadyRegistered(
            "Global app registry already initialized".to_string(),
        ));
    }

    let apps = Apps::new(installed_apps);
    apps.populate()?;

    GLOBAL_APPS
        .set(apps)
        .map_err(|_| AppError::AlreadyRegistered("Failed to set global apps".to_string()))?;

    Ok(())
}

/// Initialize the global application registry from a compile-time validated app list
///
/// This function accepts a function that returns `Vec<String>` generated by the
/// `installed_apps!` macro, providing compile-time validation of app names.
pub fn init_apps_checked<F>(app_provider: F) -> AppResult<()>
where
    F: FnOnce() -> Vec<String>,
{
    init_apps(app_provider())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_creation() {
        let config = AppConfig::new("myapp", "myapp")
            .with_verbose_name("My Application")
            .with_default_auto_field("BigAutoField");

        assert_eq!(config.name, "myapp");
        assert_eq!(config.label, "myapp");
        assert_eq!(config.verbose_name, Some("My Application".to_string()));
        assert_eq!(config.default_auto_field, Some("BigAutoField".to_string()));
    }

    #[test]
    fn test_app_config_validation() {
        let valid = AppConfig::new("myapp", "myapp");
        assert!(valid.validate_label().is_ok());

        let invalid = AppConfig::new("myapp", "my-app");
        assert!(invalid.validate_label().is_err());

        let empty = AppConfig::new("myapp", "");
        assert!(empty.validate_label().is_err());
    }

    #[test]
    fn test_apps_registry() {
        let apps = Apps::new(vec!["myapp".to_string(), "anotherapp".to_string()]);

        assert!(apps.is_installed("myapp"));
        assert!(apps.is_installed("anotherapp"));
        assert!(!apps.is_installed("notinstalled"));
    }

    #[test]
    fn test_register_app() {
        let apps = Apps::new(vec![]);
        let config = AppConfig::new("myapp", "myapp");

        assert!(apps.register(config).is_ok());
        assert!(apps.get_app_config("myapp").is_ok());
    }

    #[test]
    fn test_duplicate_registration() {
        let apps = Apps::new(vec![]);
        let config1 = AppConfig::new("myapp", "myapp");
        let config2 = AppConfig::new("myapp", "myapp");

        apps.register(config1).unwrap();
        let result = apps.register(config2);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_app_configs() {
        let apps = Apps::new(vec![]);

        apps.register(AppConfig::new("app1", "app1")).unwrap();
        apps.register(AppConfig::new("app2", "app2")).unwrap();

        let configs = apps.get_app_configs();
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_populate() {
        let apps = Apps::new(vec![]);
        assert!(!apps.is_ready());

        apps.populate().unwrap();

        assert!(apps.is_ready());
        assert!(apps.is_apps_ready());
        assert!(apps.is_models_ready());
    }

    #[test]
    fn test_populate_with_installed_apps() {
        let apps = Apps::new(vec!["myapp".to_string(), "anotherapp".to_string()]);
        assert!(!apps.is_ready());

        // Populate should create AppConfig for each installed app
        let result = apps.populate();
        assert!(result.is_ok());

        // Verify apps are ready
        assert!(apps.is_ready());
        assert!(apps.is_apps_ready());
        assert!(apps.is_models_ready());

        // Verify AppConfigs were created
        assert!(apps.get_app_config("myapp").is_ok());
        assert!(apps.get_app_config("anotherapp").is_ok());

        // Verify app configs contain correct labels
        let myapp_config = apps.get_app_config("myapp").unwrap();
        assert_eq!(myapp_config.label, "myapp");
    }
}

// ============================================================================
// Type-safe application registry (compile-time checked)
// ============================================================================

/// Trait for applications that can be accessed at compile time
///
/// Implement this trait for each application in your project.
/// The compiler will ensure that only valid application labels can be used.
///
/// # Example
///
/// ```rust
/// use reinhardt_apps::apps::AppLabel;
///
/// pub struct AuthApp;
/// impl AppLabel for AuthApp {
///     const LABEL: &'static str = "auth";
/// }
/// ```
pub trait AppLabel {
    /// The unique label for this application
    const LABEL: &'static str;
}

impl Apps {
    /// Type-safe get_app_config method
    ///
    /// This method ensures at compile time that only valid application types
    /// can be used.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_apps::apps::{Apps, AppLabel};
    ///
    /// pub struct AuthApp;
    /// impl AppLabel for AuthApp {
    ///     const LABEL: &'static str = "auth";
    /// }
    ///
    /// let apps = Apps::new(vec!["auth".to_string()]);
    // This will compile because AuthApp implements AppLabel
    /// let result = apps.get_app_config_typed::<AuthApp>();
    /// ```
    pub fn get_app_config_typed<A: AppLabel>(&self) -> AppResult<AppConfig> {
        self.get_app_config(A::LABEL)
    }

    /// Type-safe check if an application is installed
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_apps::apps::{Apps, AppLabel};
    ///
    /// pub struct AuthApp;
    /// impl AppLabel for AuthApp {
    ///     const LABEL: &'static str = "auth";
    /// }
    ///
    /// let apps = Apps::new(vec!["auth".to_string()]);
    /// assert!(apps.is_installed_typed::<AuthApp>());
    /// ```
    pub fn is_installed_typed<A: AppLabel>(&self) -> bool {
        self.is_installed(A::LABEL)
    }
}

#[cfg(test)]
mod typed_tests {
    use super::*;

    // Test application types
    struct AuthApp;
    impl AppLabel for AuthApp {
        const LABEL: &'static str = "auth";
    }

    struct ContentTypesApp;
    impl AppLabel for ContentTypesApp {
        const LABEL: &'static str = "contenttypes";
    }

    struct SessionsApp;
    impl AppLabel for SessionsApp {
        const LABEL: &'static str = "sessions";
    }

    #[test]
    fn test_typed_is_installed() {
        let apps = Apps::new(vec!["auth".to_string(), "contenttypes".to_string()]);

        assert!(apps.is_installed_typed::<AuthApp>());
        assert!(apps.is_installed_typed::<ContentTypesApp>());
        assert!(!apps.is_installed_typed::<SessionsApp>());
    }

    #[test]
    fn test_typed_get_app_config() {
        let apps = Apps::new(vec![]);
        let config = AppConfig::new("auth", "auth");
        apps.register(config).unwrap();

        let retrieved = apps.get_app_config_typed::<AuthApp>();
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().label, "auth");
    }

    #[test]
    fn test_typed_get_app_config_not_found() {
        let apps = Apps::new(vec![]);

        let result = apps.get_app_config_typed::<SessionsApp>();
        assert!(result.is_err());

        if let Err(AppError::NotFound(label)) = result {
            assert_eq!(label, "sessions");
        }
    }

    #[test]
    fn test_apps_typed_and_regular_mixed() {
        let apps = Apps::new(vec!["auth".to_string()]);
        let config = AppConfig::new("auth", "auth");
        apps.register(config).unwrap();

        // Can use both typed and regular methods
        assert!(apps.is_installed_typed::<AuthApp>());
        assert!(apps.is_installed("auth"));

        let typed = apps.get_app_config_typed::<AuthApp>().unwrap();
        let regular = apps.get_app_config("auth").unwrap();

        assert_eq!(typed.label, regular.label);
    }
}
