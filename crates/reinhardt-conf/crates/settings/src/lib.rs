//! # Settings Module
//!
//! Django-inspired settings system for Reinhardt projects.
//! This module provides configuration management for Reinhardt applications.

pub mod advanced;
pub mod builder;
pub mod env;
pub mod env_loader;
pub mod env_parser;
pub mod prelude;
pub mod profile;
pub mod sources;
pub mod validation;

// Dynamic settings (async feature required)
#[cfg(feature = "async")]
pub mod dynamic;

#[cfg(feature = "async")]
pub mod backends;

#[cfg(feature = "async")]
pub mod secrets;

// Encryption module is always available, but uses stub implementations without the feature
pub mod encryption;

#[cfg(feature = "async")]
pub mod audit;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

pub mod config;
pub mod docs;
pub mod testing;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// Re-export from advanced module
pub use advanced::{
    AdvancedSettings, CacheSettings, CorsSettings, DatabaseSettings as AdvancedDatabaseSettings,
    EmailSettings, LoggingSettings, MediaSettings, SessionSettings, SettingsError, StaticSettings,
};

/// Main settings structure for a Reinhardt project
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// Base directory of the project
    pub base_dir: PathBuf,

    /// Secret key for cryptographic signing (SECURITY WARNING: keep secret in production!)
    pub secret_key: String,

    /// Debug mode (SECURITY WARNING: don't run with debug=true in production!)
    pub debug: bool,

    /// List of allowed host/domain names
    pub allowed_hosts: Vec<String>,

    /// List of installed applications
    pub installed_apps: Vec<String>,

    /// List of middleware classes
    pub middleware: Vec<String>,

    /// Root URL configuration module
    pub root_urlconf: String,

    /// Database configurations
    pub databases: HashMap<String, DatabaseConfig>,

    /// Template configurations
    pub templates: Vec<TemplateConfig>,

    /// Static files URL prefix
    pub static_url: String,

    /// Static files root directory
    pub static_root: Option<PathBuf>,

    /// Media files URL prefix
    pub media_url: String,

    /// Media files root directory
    pub media_root: Option<PathBuf>,

    /// Language code
    pub language_code: String,

    /// Time zone
    pub time_zone: String,

    /// Enable internationalization
    pub use_i18n: bool,

    /// Use timezone-aware datetimes
    pub use_tz: bool,

    /// Default auto field for models
    pub default_auto_field: String,

    /// HTTPS/Security settings
    /// Header name and value to use for identifying secure requests behind a proxy
    /// Example: Some(("HTTP_X_FORWARDED_PROTO", "https"))
    pub secure_proxy_ssl_header: Option<(String, String)>,

    /// Redirect all HTTP requests to HTTPS
    pub secure_ssl_redirect: bool,

    /// Seconds to set HSTS max-age header
    pub secure_hsts_seconds: Option<u64>,

    /// Include subdomains in HSTS policy
    pub secure_hsts_include_subdomains: bool,

    /// Include preload directive in HSTS header
    pub secure_hsts_preload: bool,

    /// Only send cookies over HTTPS
    pub session_cookie_secure: bool,

    /// Only send CSRF cookie over HTTPS
    pub csrf_cookie_secure: bool,

    /// Automatically append trailing slashes to URLs
    pub append_slash: bool,
}

impl Settings {
    /// Create a new Settings instance with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::Settings;
    /// use std::path::PathBuf;
    ///
    /// let settings = Settings::new(
    ///     PathBuf::from("/app"),
    ///     "my-secret-key-12345".to_string()
    /// );
    ///
    /// assert_eq!(settings.base_dir, PathBuf::from("/app"));
    /// assert_eq!(settings.secret_key, "my-secret-key-12345");
    /// assert_eq!(settings.debug, true);
    /// assert_eq!(settings.time_zone, "UTC");
    /// assert!(settings.installed_apps.contains(&"reinhardt.contrib.admin".to_string()));
    /// ```
    pub fn new(base_dir: PathBuf, secret_key: String) -> Self {
        Self {
            base_dir,
            secret_key,
            debug: true,
            allowed_hosts: vec![],
            installed_apps: vec![
                "reinhardt.contrib.admin".to_string(),
                "reinhardt.contrib.auth".to_string(),
                "reinhardt.contrib.contenttypes".to_string(),
                "reinhardt.contrib.sessions".to_string(),
                "reinhardt.contrib.messages".to_string(),
                "reinhardt.contrib.staticfiles".to_string(),
            ],
            middleware: vec![
                "reinhardt.middleware.security.SecurityMiddleware".to_string(),
                "reinhardt.contrib.sessions.middleware.SessionMiddleware".to_string(),
                "reinhardt.middleware.common.CommonMiddleware".to_string(),
                "reinhardt.middleware.csrf.CsrfViewMiddleware".to_string(),
                "reinhardt.contrib.auth.middleware.AuthenticationMiddleware".to_string(),
                "reinhardt.contrib.messages.middleware.MessageMiddleware".to_string(),
                "reinhardt.middleware.clickjacking.XFrameOptionsMiddleware".to_string(),
            ],
            root_urlconf: String::new(),
            databases: {
                let mut dbs = HashMap::new();
                dbs.insert("default".to_string(), DatabaseConfig::default());
                dbs
            },
            templates: vec![TemplateConfig::default()],
            static_url: "/static/".to_string(),
            static_root: None,
            media_url: "/media/".to_string(),
            media_root: None,
            language_code: "en-us".to_string(),
            time_zone: "UTC".to_string(),
            use_i18n: true,
            use_tz: true,
            default_auto_field: "reinhardt.db.models.BigAutoField".to_string(),
            secure_proxy_ssl_header: None,
            secure_ssl_redirect: false,
            secure_hsts_seconds: None,
            secure_hsts_include_subdomains: false,
            secure_hsts_preload: false,
            session_cookie_secure: false,
            csrf_cookie_secure: false,
            append_slash: true,
        }
    }
    /// Set the root URL configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::Settings;
    /// use std::path::PathBuf;
    ///
    /// let settings = Settings::new(
    ///     PathBuf::from("/app"),
    ///     "secret".to_string()
    /// ).with_root_urlconf("myapp.urls");
    ///
    /// assert_eq!(settings.root_urlconf, "myapp.urls");
    /// ```
    pub fn with_root_urlconf(mut self, root_urlconf: impl Into<String>) -> Self {
        self.root_urlconf = root_urlconf.into();
        self
    }
    /// Add an installed app
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::Settings;
    ///
    /// let mut settings = Settings::default();
    /// let initial_count = settings.installed_apps.len();
    /// settings.add_app("myapp");
    ///
    /// assert_eq!(settings.installed_apps.len(), initial_count + 1);
    /// assert!(settings.installed_apps.contains(&"myapp".to_string()));
    /// ```
    pub fn add_app(&mut self, app: impl Into<String>) {
        self.installed_apps.push(app.into());
    }
    /// Create settings with a compile-time validated app list
    ///
    /// This method accepts a function that returns `Vec<String>` generated by the
    /// `installed_apps!` macro, providing compile-time validation of app names.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::Settings;
    ///
    /// let settings = Settings::default()
    ///     .with_validated_apps(|| vec![
    ///         "reinhardt.contrib.admin".to_string(),
    ///         "myapp".to_string(),
    ///     ]);
    ///
    /// assert_eq!(settings.installed_apps.len(), 2);
    /// assert!(settings.installed_apps.contains(&"myapp".to_string()));
    /// ```
    pub fn with_validated_apps<F>(mut self, app_provider: F) -> Self
    where
        F: FnOnce() -> Vec<String>,
    {
        self.installed_apps = app_provider();
        self
    }
    /// Add middleware
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::Settings;
    ///
    /// let mut settings = Settings::default();
    /// let initial_count = settings.middleware.len();
    /// settings.add_middleware("myapp.middleware.CustomMiddleware");
    ///
    /// assert_eq!(settings.middleware.len(), initial_count + 1);
    /// assert!(settings.middleware.contains(&"myapp.middleware.CustomMiddleware".to_string()));
    /// ```
    pub fn add_middleware(&mut self, middleware: impl Into<String>) {
        self.middleware.push(middleware.into());
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new(
            PathBuf::from("."),
            "insecure-change-this-in-production".to_string(),
        )
    }
}

/// Database configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database engine/backend
    pub engine: String,

    /// Database name or path
    pub name: String,

    /// Database user (if applicable)
    pub user: Option<String>,

    /// Database password (if applicable)
    pub password: Option<String>,

    /// Database host (if applicable)
    pub host: Option<String>,

    /// Database port (if applicable)
    pub port: Option<u16>,

    /// Additional options
    pub options: HashMap<String, String>,
}

impl DatabaseConfig {
    /// Create a SQLite database configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::DatabaseConfig;
    ///
    /// let db = DatabaseConfig::sqlite("myapp.db");
    ///
    /// assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
    /// assert_eq!(db.name, "myapp.db");
    /// assert!(db.user.is_none());
    /// assert!(db.password.is_none());
    /// ```
    pub fn sqlite(name: impl Into<String>) -> Self {
        Self {
            engine: "reinhardt.db.backends.sqlite3".to_string(),
            name: name.into(),
            user: None,
            password: None,
            host: None,
            port: None,
            options: HashMap::new(),
        }
    }
    /// Create a PostgreSQL database configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::DatabaseConfig;
    ///
    /// let db = DatabaseConfig::postgresql("mydb", "admin", "password123", "localhost", 5432);
    ///
    /// assert_eq!(db.engine, "reinhardt.db.backends.postgresql");
    /// assert_eq!(db.name, "mydb");
    /// assert_eq!(db.user, Some("admin".to_string()));
    /// assert_eq!(db.password, Some("password123".to_string()));
    /// assert_eq!(db.host, Some("localhost".to_string()));
    /// assert_eq!(db.port, Some(5432));
    /// ```
    pub fn postgresql(
        name: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            engine: "reinhardt.db.backends.postgresql".to_string(),
            name: name.into(),
            user: Some(user.into()),
            password: Some(password.into()),
            host: Some(host.into()),
            port: Some(port),
            options: HashMap::new(),
        }
    }
    /// Create a MySQL database configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::DatabaseConfig;
    ///
    /// let db = DatabaseConfig::mysql("mydb", "root", "password123", "localhost", 3306);
    ///
    /// assert_eq!(db.engine, "reinhardt.db.backends.mysql");
    /// assert_eq!(db.name, "mydb");
    /// assert_eq!(db.user, Some("root".to_string()));
    /// assert_eq!(db.password, Some("password123".to_string()));
    /// assert_eq!(db.host, Some("localhost".to_string()));
    /// assert_eq!(db.port, Some(3306));
    /// ```
    pub fn mysql(
        name: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            engine: "reinhardt.db.backends.mysql".to_string(),
            name: name.into(),
            user: Some(user.into()),
            password: Some(password.into()),
            host: Some(host.into()),
            port: Some(port),
            options: HashMap::new(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::sqlite("db.sqlite3".to_string())
    }
}

/// Template engine configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Template backend/engine
    pub backend: String,

    /// Directories to search for templates
    pub dirs: Vec<PathBuf>,

    /// Search for templates in app directories
    pub app_dirs: bool,

    /// Template engine options
    pub options: HashMap<String, serde_json::Value>,
}

impl TemplateConfig {
    /// Create a new template configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::TemplateConfig;
    ///
    /// let config = TemplateConfig::new("reinhardt.template.backends.jinja2.Jinja2");
    ///
    /// assert_eq!(config.backend, "reinhardt.template.backends.jinja2.Jinja2");
    /// assert!(config.app_dirs);
    /// assert_eq!(config.dirs.len(), 0);
    /// ```
    pub fn new(backend: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            dirs: vec![],
            app_dirs: true,
            options: HashMap::new(),
        }
    }
    /// Add a template directory
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::TemplateConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = TemplateConfig::new("reinhardt.template.backends.jinja2.Jinja2")
    ///     .add_dir("/app/templates");
    ///
    /// assert_eq!(config.dirs.len(), 1);
    /// assert_eq!(config.dirs[0], PathBuf::from("/app/templates"));
    /// ```
    pub fn add_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.push(dir.into());
        self
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        let mut options = HashMap::new();
        options.insert(
            "context_processors".to_string(),
            serde_json::json!([
                "reinhardt.template.context_processors.request",
                "reinhardt.contrib.auth.context_processors.auth",
                "reinhardt.contrib.messages.context_processors.messages",
            ]),
        );

        Self {
            backend: "reinhardt.template.backends.jinja2.Jinja2".to_string(),
            dirs: vec![],
            app_dirs: true,
            options,
        }
    }
}

/// Middleware configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    /// Full path to the middleware class
    pub path: String,

    /// Middleware options
    pub options: HashMap<String, serde_json::Value>,
}

impl MiddlewareConfig {
    /// Create a new middleware configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::MiddlewareConfig;
    ///
    /// let middleware = MiddlewareConfig::new("myapp.middleware.CustomMiddleware");
    ///
    /// assert_eq!(middleware.path, "myapp.middleware.CustomMiddleware");
    /// assert_eq!(middleware.options.len(), 0);
    /// ```
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            options: HashMap::new(),
        }
    }
    /// Add an option to the middleware
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::MiddlewareConfig;
    ///
    /// let middleware = MiddlewareConfig::new("myapp.middleware.CustomMiddleware")
    ///     .with_option("timeout", serde_json::json!(30));
    ///
    /// assert_eq!(middleware.options.get("timeout"), Some(&serde_json::json!(30)));
    /// ```
    pub fn with_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.options.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default_unit() {
        let settings = Settings::default();
        assert_eq!(settings.debug, true);
        assert_eq!(settings.language_code, "en-us");
        assert_eq!(settings.time_zone, "UTC");
    }

    #[test]
    fn test_settings_db_config_sqlite() {
        let db = DatabaseConfig::sqlite("test.db");
        assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
        assert_eq!(db.name, "test.db");
        assert!(db.user.is_none());
    }

    #[test]
    fn test_settings_db_config_postgresql() {
        let db = DatabaseConfig::postgresql("testdb", "user", "pass", "localhost", 5432);
        assert_eq!(db.engine, "reinhardt.db.backends.postgresql");
        assert_eq!(db.name, "testdb");
        assert_eq!(db.user, Some("user".to_string()));
        assert_eq!(db.port, Some(5432));
    }

    #[test]
    fn test_template_config() {
        let config = TemplateConfig::default();
        assert!(config.app_dirs);
        assert_eq!(config.backend, "reinhardt.template.backends.jinja2.Jinja2");
    }

    #[test]
    fn test_middleware_config() {
        let middleware = MiddlewareConfig::new("reinhardt.middleware.TestMiddleware")
            .with_option("enabled", serde_json::json!(true));

        assert_eq!(middleware.path, "reinhardt.middleware.TestMiddleware");
        assert_eq!(
            middleware.options.get("enabled"),
            Some(&serde_json::json!(true))
        );
    }
}
