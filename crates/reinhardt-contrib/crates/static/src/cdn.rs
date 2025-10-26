//! CDN integration helpers
//!
//! Provides utilities for integrating with Content Delivery Networks (CDNs)
//! such as CloudFront, Fastly, and Cloudflare.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CDN provider type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CdnProvider {
    /// Amazon CloudFront
    CloudFront,
    /// Fastly
    Fastly,
    /// Cloudflare
    Cloudflare,
    /// Custom CDN provider
    Custom(String),
}

/// CDN configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConfig {
    /// Whether CDN integration is enabled
    pub enabled: bool,
    /// CDN provider
    pub provider: CdnProvider,
    /// CDN base URL (e.g., "https://d1234567890.cloudfront.net")
    pub base_url: String,
    /// Path prefix on CDN (e.g., "/static")
    pub path_prefix: Option<String>,
    /// Custom headers to add to CDN requests
    pub custom_headers: HashMap<String, String>,
    /// Whether to use HTTPS
    pub use_https: bool,
}

impl CdnConfig {
    /// Create a new CDN configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// );
    /// ```
    pub fn new(provider: CdnProvider, base_url: String) -> Self {
        Self {
            enabled: true,
            provider,
            base_url: base_url.trim_end_matches('/').to_string(),
            path_prefix: None,
            custom_headers: HashMap::new(),
            use_https: true,
        }
    }

    /// Disable CDN integration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            provider: CdnProvider::Custom("none".to_string()),
            base_url: String::new(),
            path_prefix: None,
            custom_headers: HashMap::new(),
            use_https: true,
        }
    }

    /// Set path prefix
    pub fn with_path_prefix(mut self, prefix: String) -> Self {
        self.path_prefix = Some(prefix.trim_start_matches('/').to_string());
        self
    }

    /// Add custom header
    pub fn with_custom_header(mut self, key: String, value: String) -> Self {
        self.custom_headers.insert(key, value);
        self
    }

    /// Disable HTTPS
    pub fn without_https(mut self) -> Self {
        self.use_https = false;
        self
    }
}

/// CDN URL generator
pub struct CdnUrlGenerator {
    config: CdnConfig,
}

impl CdnUrlGenerator {
    /// Create a new CDN URL generator
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnUrlGenerator, CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// );
    /// let generator = CdnUrlGenerator::new(config);
    /// ```
    pub fn new(config: CdnConfig) -> Self {
        Self { config }
    }

    /// Generate a CDN URL for a given path
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnUrlGenerator, CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// ).with_path_prefix("static".to_string());
    ///
    /// let generator = CdnUrlGenerator::new(config);
    /// let url = generator.generate_url("/css/style.css");
    ///
    /// assert_eq!(url, "https://d1234567890.cloudfront.net/static/css/style.css");
    /// ```
    pub fn generate_url(&self, path: &str) -> String {
        if !self.config.enabled {
            return path.to_string();
        }

        let scheme = if self.config.use_https {
            "https"
        } else {
            "http"
        };
        let path = path.trim_start_matches('/');

        let full_path = if let Some(prefix) = &self.config.path_prefix {
            format!("{}/{}", prefix.trim_end_matches('/'), path)
        } else {
            path.to_string()
        };

        format!("{}://{}/{}", scheme, self.config.base_url, full_path)
    }

    /// Generate URLs for multiple paths
    pub fn generate_urls(&self, paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| self.generate_url(p)).collect()
    }

    /// Generate a versioned URL with query parameter
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnUrlGenerator, CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// );
    ///
    /// let generator = CdnUrlGenerator::new(config);
    /// let url = generator.generate_versioned_url("/css/style.css", "v1.2.3");
    ///
    /// assert_eq!(url, "https://d1234567890.cloudfront.net/css/style.css?v=v1.2.3");
    /// ```
    pub fn generate_versioned_url(&self, path: &str, version: &str) -> String {
        let base_url = self.generate_url(path);
        format!("{}?v={}", base_url, version)
    }
}

/// CDN cache invalidation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnInvalidationRequest {
    /// Paths to invalidate
    pub paths: Vec<String>,
    /// Caller reference (unique identifier for this request)
    pub caller_reference: Option<String>,
}

impl CdnInvalidationRequest {
    /// Create a new invalidation request
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::CdnInvalidationRequest;
    ///
    /// let request = CdnInvalidationRequest::new(vec![
    ///     "/css/style.css".to_string(),
    ///     "/js/app.js".to_string(),
    /// ]);
    /// ```
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            paths,
            caller_reference: None,
        }
    }

    /// Set caller reference
    pub fn with_caller_reference(mut self, reference: String) -> Self {
        self.caller_reference = Some(reference);
        self
    }

    /// Add path to invalidate
    pub fn add_path(&mut self, path: String) {
        self.paths.push(path);
    }

    /// Add multiple paths
    pub fn add_paths(&mut self, paths: Vec<String>) {
        self.paths.extend(paths);
    }
}

/// CDN purge helper (simplified API for common operations)
pub struct CdnPurgeHelper {
    config: CdnConfig,
}

impl CdnPurgeHelper {
    /// Create a new purge helper
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnPurgeHelper, CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// );
    /// let helper = CdnPurgeHelper::new(config);
    /// ```
    pub fn new(config: CdnConfig) -> Self {
        Self { config }
    }

    /// Create an invalidation request for given paths
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_static::cdn::{CdnPurgeHelper, CdnConfig, CdnProvider};
    ///
    /// let config = CdnConfig::new(
    ///     CdnProvider::CloudFront,
    ///     "d1234567890.cloudfront.net".to_string(),
    /// );
    /// let helper = CdnPurgeHelper::new(config);
    ///
    /// let request = helper.create_invalidation_request(vec![
    ///     "/css/style.css".to_string(),
    ///     "/js/app.js".to_string(),
    /// ]);
    /// ```
    pub fn create_invalidation_request(&self, paths: Vec<String>) -> CdnInvalidationRequest {
        CdnInvalidationRequest::new(paths)
    }

    /// Create an invalidation request for all files matching a pattern
    pub fn create_wildcard_invalidation(&self, pattern: &str) -> CdnInvalidationRequest {
        CdnInvalidationRequest::new(vec![pattern.to_string()])
    }

    /// Get the CDN purge endpoint URL
    ///
    /// # Implementation Status
    ///
    /// Returns the API endpoint URL for each CDN provider.
    /// For production use with actual purging, integrate with the CDN provider's SDK:
    /// - CloudFront: Use `aws-sdk-cloudfront` crate
    /// - Fastly: Use `fastly` crate or HTTP API
    /// - Cloudflare: Use `cloudflare` crate or HTTP API
    ///
    /// # Note
    ///
    /// The returned URLs contain placeholders (e.g., `{distribution-id}`, `{service-id}`)
    /// that must be replaced with actual values from your CDN configuration.
    pub fn get_purge_endpoint(&self) -> String {
        match &self.config.provider {
            CdnProvider::CloudFront => {
                // CloudFront invalidation endpoint
                // Replace {distribution-id} with your actual distribution ID
                "https://cloudfront.amazonaws.com/2020-05-31/distribution/{distribution-id}/invalidation".to_string()
            }
            CdnProvider::Fastly => {
                // Fastly purge endpoint
                // Replace {service-id} with your actual service ID
                "https://api.fastly.com/service/{service-id}/purge".to_string()
            }
            CdnProvider::Cloudflare => {
                // Cloudflare purge cache endpoint
                // Replace {zone-id} with your actual zone ID
                "https://api.cloudflare.com/client/v4/zones/{zone-id}/purge_cache".to_string()
            }
            CdnProvider::Custom(name) => {
                // Custom CDN endpoint
                format!("custom://{}/purge", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdn_config_creation() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        );

        assert!(config.enabled);
        assert_eq!(config.provider, CdnProvider::CloudFront);
        assert_eq!(config.base_url, "d1234567890.cloudfront.net");
        assert!(config.use_https);
    }

    #[test]
    fn test_cdn_url_generation() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        );
        let generator = CdnUrlGenerator::new(config);

        let url = generator.generate_url("/css/style.css");
        assert_eq!(url, "https://d1234567890.cloudfront.net/css/style.css");
    }

    #[test]
    fn test_cdn_url_generation_with_prefix() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        )
        .with_path_prefix("static".to_string());

        let generator = CdnUrlGenerator::new(config);
        let url = generator.generate_url("/css/style.css");

        assert_eq!(
            url,
            "https://d1234567890.cloudfront.net/static/css/style.css"
        );
    }

    #[test]
    fn test_cdn_url_generation_without_https() {
        let config =
            CdnConfig::new(CdnProvider::Fastly, "example.fastly.net".to_string()).without_https();

        let generator = CdnUrlGenerator::new(config);
        let url = generator.generate_url("/image.png");

        assert_eq!(url, "http://example.fastly.net/image.png");
    }

    #[test]
    fn test_versioned_url_generation() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        );
        let generator = CdnUrlGenerator::new(config);

        let url = generator.generate_versioned_url("/css/style.css", "v1.2.3");
        assert_eq!(
            url,
            "https://d1234567890.cloudfront.net/css/style.css?v=v1.2.3"
        );
    }

    #[test]
    fn test_multiple_urls_generation() {
        let config = CdnConfig::new(CdnProvider::Cloudflare, "cdn.example.com".to_string());
        let generator = CdnUrlGenerator::new(config);

        let paths = vec!["/css/style.css", "/js/app.js", "/img/logo.png"];
        let urls = generator.generate_urls(&paths);

        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "https://cdn.example.com/css/style.css");
        assert_eq!(urls[1], "https://cdn.example.com/js/app.js");
        assert_eq!(urls[2], "https://cdn.example.com/img/logo.png");
    }

    #[test]
    fn test_disabled_cdn_returns_original_path() {
        let config = CdnConfig::disabled();
        let generator = CdnUrlGenerator::new(config);

        let url = generator.generate_url("/css/style.css");
        assert_eq!(url, "/css/style.css");
    }

    #[test]
    fn test_invalidation_request_creation() {
        let request = CdnInvalidationRequest::new(vec![
            "/css/style.css".to_string(),
            "/js/app.js".to_string(),
        ]);

        assert_eq!(request.paths.len(), 2);
        assert!(request.caller_reference.is_none());
    }

    #[test]
    fn test_invalidation_request_with_caller_reference() {
        let request = CdnInvalidationRequest::new(vec!["/css/style.css".to_string()])
            .with_caller_reference("unique-id-123".to_string());

        assert_eq!(request.caller_reference, Some("unique-id-123".to_string()));
    }

    #[test]
    fn test_add_paths_to_invalidation_request() {
        let mut request = CdnInvalidationRequest::new(vec!["/css/style.css".to_string()]);
        request.add_path("/js/app.js".to_string());
        request.add_paths(vec![
            "/img/logo.png".to_string(),
            "/fonts/font.woff2".to_string(),
        ]);

        assert_eq!(request.paths.len(), 4);
    }

    #[test]
    fn test_purge_helper_creates_request() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        );
        let helper = CdnPurgeHelper::new(config);

        let request = helper.create_invalidation_request(vec![
            "/css/style.css".to_string(),
            "/js/app.js".to_string(),
        ]);

        assert_eq!(request.paths.len(), 2);
    }

    #[test]
    fn test_wildcard_invalidation() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        );
        let helper = CdnPurgeHelper::new(config);

        let request = helper.create_wildcard_invalidation("/css/*");
        assert_eq!(request.paths, vec!["/css/*"]);
    }

    #[test]
    fn test_purge_endpoints() {
        // Test CloudFront endpoint
        let config = CdnConfig::new(CdnProvider::CloudFront, "example.com".to_string());
        let helper = CdnPurgeHelper::new(config);
        let endpoint = helper.get_purge_endpoint();
        assert!(endpoint.contains("cloudfront.amazonaws.com"));
        assert!(endpoint.contains("invalidation"));

        // Test Fastly endpoint
        let config = CdnConfig::new(CdnProvider::Fastly, "example.com".to_string());
        let helper = CdnPurgeHelper::new(config);
        let endpoint = helper.get_purge_endpoint();
        assert!(endpoint.contains("api.fastly.com"));
        assert!(endpoint.contains("purge"));

        // Test Cloudflare endpoint
        let config = CdnConfig::new(CdnProvider::Cloudflare, "example.com".to_string());
        let helper = CdnPurgeHelper::new(config);
        let endpoint = helper.get_purge_endpoint();
        assert!(endpoint.contains("api.cloudflare.com"));
        assert!(endpoint.contains("purge_cache"));

        // Test Custom provider
        let config = CdnConfig::new(
            CdnProvider::Custom("mycdn".to_string()),
            "example.com".to_string(),
        );
        let helper = CdnPurgeHelper::new(config);
        let endpoint = helper.get_purge_endpoint();
        assert!(endpoint.contains("custom://mycdn/purge"));
    }

    #[test]
    fn test_custom_provider() {
        let config = CdnConfig::new(
            CdnProvider::Custom("my-cdn".to_string()),
            "cdn.mycompany.com".to_string(),
        );

        assert_eq!(config.provider, CdnProvider::Custom("my-cdn".to_string()));
    }

    #[test]
    fn test_custom_headers() {
        let config = CdnConfig::new(
            CdnProvider::CloudFront,
            "d1234567890.cloudfront.net".to_string(),
        )
        .with_custom_header("X-Custom-Header".to_string(), "value".to_string())
        .with_custom_header("Authorization".to_string(), "Bearer token".to_string());

        assert_eq!(config.custom_headers.len(), 2);
        assert_eq!(
            config.custom_headers.get("X-Custom-Header"),
            Some(&"value".to_string())
        );
    }
}
