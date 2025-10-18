//! Static files filters for Askama templates
//!
//! Provides filters for generating URLs to static files, similar to Django's
//! `{% static %}` template tag.

use std::path::Path;
use std::sync::RwLock;

/// Global static files configuration
static STATIC_CONFIG: RwLock<Option<StaticConfig>> = RwLock::new(None);

#[derive(Debug, Clone)]
pub struct StaticConfig {
    /// Base URL for static files (e.g., "/static/")
    pub static_url: String,
    /// Whether to use hashed filenames
    pub use_manifest: bool,
    /// Manifest mapping original paths to hashed paths
    pub manifest: std::collections::HashMap<String, String>,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            static_url: "/static/".to_string(),
            use_manifest: false,
            manifest: std::collections::HashMap::new(),
        }
    }
}

/// Initialize the global static configuration
///
/// This should be called once at application startup.
///
/// # Example
///
/// ```rust
/// use reinhardt_templates::static_filters::{StaticConfig, init_static_config};
///
/// init_static_config(StaticConfig {
///     static_url: "/static/".to_string(),
///     use_manifest: false,
///     manifest: Default::default(),
/// });
/// ```
pub fn init_static_config(config: StaticConfig) {
    let mut guard = STATIC_CONFIG.write().unwrap();
    *guard = Some(config);
}

/// Get the current static configuration
fn get_static_config() -> StaticConfig {
    let guard = STATIC_CONFIG.read().unwrap();
    guard.clone().unwrap_or_default()
}

/// Generate a URL for a static file
///
/// This is the main filter function that can be used in Askama templates.
/// It handles both regular and hashed static file URLs.
///
/// # Arguments
///
/// * `path` - The relative path to the static file (e.g., "css/style.css")
///
/// # Returns
///
/// The full URL to the static file (e.g., "/static/css/style.css" or
/// "/static/css/style.abc123.css" if using hashed filenames)
///
/// # Example in Askama template
///
/// ```html
/// <link rel="stylesheet" href="{{ "css/style.css"|static }}">
/// <script src="{{ "js/app.js"|static }}"></script>
/// <img src="{{ "images/logo.png"|static }}" alt="Logo">
/// ```
pub fn static_filter(path: &str) -> askama::Result<String> {
    let config = get_static_config();

    // Normalize the path (remove leading slash if present)
    let normalized_path = path.trim_start_matches('/');

    // Look up hashed version if manifest is enabled
    let final_path = if config.use_manifest {
        config
            .manifest
            .get(normalized_path)
            .map(|s| s.as_str())
            .unwrap_or(normalized_path)
    } else {
        normalized_path
    };

    // Construct the full URL
    let static_url = config.static_url.trim_end_matches('/');
    Ok(format!("{}/{}", static_url, final_path))
}

/// Join multiple path components into a single path
///
/// Useful for dynamically constructing static file paths.
///
/// # Example in Askama template
///
/// ```html
/// {# Construct path dynamically #}
/// {% let image_path = "images/" ~ image_name %}
/// <img src="{{ image_path|static }}" alt="Dynamic image">
/// ```
pub fn static_path_join(base: &str, path: &str) -> askama::Result<String> {
    let joined = Path::new(base).join(path);
    Ok(joined.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_static_filter_default_config() {
        // Reset config for test
        let config = StaticConfig::default();
        init_static_config(config);

        assert_eq!(
            static_filter("css/style.css").unwrap(),
            "/static/css/style.css"
        );
        assert_eq!(static_filter("js/app.js").unwrap(), "/static/js/app.js");
        assert_eq!(
            static_filter("images/logo.png").unwrap(),
            "/static/images/logo.png"
        );
    }

    #[test]
    fn test_static_filter_leading_slash() {
        let config = StaticConfig::default();
        init_static_config(config);

        // Leading slash should be removed
        assert_eq!(
            static_filter("/css/style.css").unwrap(),
            "/static/css/style.css"
        );
    }

    #[test]
    fn test_static_filter_custom_url() {
        let config = StaticConfig {
            static_url: "/assets/".to_string(),
            use_manifest: false,
            manifest: HashMap::new(),
        };
        init_static_config(config);

        assert_eq!(
            static_filter("css/style.css").unwrap(),
            "/assets/css/style.css"
        );

        // Reset to default after test
        init_static_config(StaticConfig::default());
    }

    #[test]
    fn test_static_filter_with_manifest() {
        let mut manifest = HashMap::new();
        manifest.insert(
            "css/style.css".to_string(),
            "css/style.abc123.css".to_string(),
        );
        manifest.insert("js/app.js".to_string(), "js/app.def456.js".to_string());

        let config = StaticConfig {
            static_url: "/static/".to_string(),
            use_manifest: true,
            manifest,
        };
        init_static_config(config);

        assert_eq!(
            static_filter("css/style.css").unwrap(),
            "/static/css/style.abc123.css"
        );
        assert_eq!(
            static_filter("js/app.js").unwrap(),
            "/static/js/app.def456.js"
        );

        // Non-hashed file should use original path
        assert_eq!(
            static_filter("images/logo.png").unwrap(),
            "/static/images/logo.png"
        );
    }

    #[test]
    fn test_static_filter_url_without_trailing_slash() {
        let config = StaticConfig {
            static_url: "/static".to_string(),
            use_manifest: false,
            manifest: HashMap::new(),
        };
        init_static_config(config);

        assert_eq!(
            static_filter("css/style.css").unwrap(),
            "/static/css/style.css"
        );
    }

    #[test]
    fn test_static_path_join() {
        assert_eq!(
            static_path_join("images", "logo.png").unwrap(),
            "images/logo.png"
        );
        assert_eq!(
            static_path_join("css/vendor", "bootstrap.css").unwrap(),
            "css/vendor/bootstrap.css"
        );
    }
}
