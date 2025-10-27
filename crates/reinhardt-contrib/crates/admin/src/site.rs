//! Admin site management
//!
//! The `AdminSite` is the central registry for all admin models and provides
//! routing, authentication, and rendering functionality.

use crate::{AdminError, AdminResult, ModelAdmin, ModelAdminConfig};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// The main admin site that manages all registered models
///
/// # Examples
///
/// ```
/// use reinhardt_admin::AdminSite;
///
/// let admin = AdminSite::new("My Application");
/// assert_eq!(admin.name(), "My Application");
/// ```
pub struct AdminSite {
    /// Site name displayed in the admin interface
    name: String,

    /// URL prefix for admin routes (default: "/admin")
    url_prefix: String,

    /// Registry of model admins indexed by model name
    registry: Arc<DashMap<String, Arc<dyn ModelAdmin>>>,

    /// Site-level configuration
    config: Arc<RwLock<AdminSiteConfig>>,
}

/// Configuration for the admin site
#[derive(Debug, Clone)]
pub struct AdminSiteConfig {
    /// Site title shown in browser tab
    pub site_title: String,

    /// Header text shown at the top of admin pages
    pub site_header: String,

    /// Index page title
    pub index_title: String,

    /// Items per page in list views
    pub list_per_page: usize,

    /// Enable search functionality
    pub enable_search: bool,

    /// Enable filtering functionality
    pub enable_filters: bool,
}

impl Default for AdminSiteConfig {
    fn default() -> Self {
        Self {
            site_title: "Admin Panel".to_string(),
            site_header: "Administration".to_string(),
            index_title: "Dashboard".to_string(),
            list_per_page: 100,
            enable_search: true,
            enable_filters: true,
        }
    }
}

impl AdminSite {
    /// Create a new admin site
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("E-commerce Admin");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url_prefix: "/admin".to_string(),
            registry: Arc::new(DashMap::new()),
            config: Arc::new(RwLock::new(AdminSiteConfig::default())),
        }
    }

    /// Get the site name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("My Admin");
    /// assert_eq!(admin.name(), "My Admin");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the URL prefix for admin routes
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let mut admin = AdminSite::new("Admin");
    /// admin.set_url_prefix("/manage");
    /// assert_eq!(admin.url_prefix(), "/manage");
    /// ```
    pub fn set_url_prefix(&mut self, prefix: impl Into<String>) {
        self.url_prefix = prefix.into();
    }

    /// Get the URL prefix
    pub fn url_prefix(&self) -> &str {
        &self.url_prefix
    }

    /// Configure the admin site
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::{AdminSite, site::AdminSiteConfig};
    ///
    /// let admin = AdminSite::new("Admin");
    /// admin.configure(|config| {
    ///     config.site_title = "My Custom Admin".to_string();
    ///     config.list_per_page = 50;
    /// });
    /// ```
    pub fn configure<F>(&self, f: F)
    where
        F: FnOnce(&mut AdminSiteConfig),
    {
        let mut config = self.config.write();
        f(&mut config);
    }

    /// Get the current configuration
    pub fn config(&self) -> AdminSiteConfig {
        self.config.read().clone()
    }

    /// Register a model with the admin site
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_admin::{AdminSite, ModelAdminConfig};
    ///
    /// let admin = AdminSite::new("Admin");
    ///
    /// let user_admin = ModelAdminConfig::builder()
    ///     .model_name("User")
    ///     .list_display(vec!["id", "username", "email"])
    ///     .build();
    ///
    /// admin.register("User", user_admin);
    /// ```
    pub fn register(
        &self,
        model_name: impl Into<String>,
        admin: impl ModelAdmin + 'static,
    ) -> AdminResult<()> {
        let model_name = model_name.into();
        self.registry.insert(model_name.clone(), Arc::new(admin));
        Ok(())
    }

    /// Unregister a model from the admin site
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// // ... register User ...
    /// admin.unregister("User");
    /// ```
    pub fn unregister(&self, model_name: &str) -> AdminResult<()> {
        self.registry
            .remove(model_name)
            .ok_or_else(|| AdminError::ModelNotRegistered(model_name.to_string()))?;
        Ok(())
    }

    /// Check if a model is registered
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// assert!(!admin.is_registered("User"));
    /// ```
    pub fn is_registered(&self, model_name: &str) -> bool {
        self.registry.contains_key(model_name)
    }

    /// Get the admin for a specific model
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// // ... register User ...
    /// let user_admin = admin.get_model_admin("User").unwrap();
    /// ```
    pub fn get_model_admin(&self, model_name: &str) -> AdminResult<Arc<dyn ModelAdmin>> {
        self.registry
            .get(model_name)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| AdminError::ModelNotRegistered(model_name.to_string()))
    }

    /// Get all registered model names
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// assert_eq!(admin.registered_models().len(), 0);
    /// ```
    pub fn registered_models(&self) -> Vec<String> {
        self.registry.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get the number of registered models
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// assert_eq!(admin.model_count(), 0);
    /// ```
    pub fn model_count(&self) -> usize {
        self.registry.len()
    }

    /// Clear all registered models
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_admin::AdminSite;
    ///
    /// let admin = AdminSite::new("Admin");
    /// admin.clear();
    /// assert_eq!(admin.model_count(), 0);
    /// ```
    pub fn clear(&self) {
        self.registry.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModelAdminConfig;

    #[test]
    fn test_admin_site_creation() {
        let admin = AdminSite::new("Test Admin");
        assert_eq!(admin.name(), "Test Admin");
        assert_eq!(admin.url_prefix(), "/admin");
        assert_eq!(admin.model_count(), 0);
    }

    #[test]
    fn test_url_prefix() {
        let mut admin = AdminSite::new("Admin");
        admin.set_url_prefix("/manage");
        assert_eq!(admin.url_prefix(), "/manage");
    }

    #[test]
    fn test_configuration() {
        let admin = AdminSite::new("Admin");
        admin.configure(|config| {
            config.site_title = "Custom Title".to_string();
            config.list_per_page = 25;
        });

        let config = admin.config();
        assert_eq!(config.site_title, "Custom Title");
        assert_eq!(config.list_per_page, 25);
    }

    #[test]
    fn test_register_and_unregister() {
        let admin = AdminSite::new("Admin");
        let model_admin = ModelAdminConfig::new("User");

        assert!(!admin.is_registered("User"));

        admin.register("User", model_admin).unwrap();
        assert!(admin.is_registered("User"));
        assert_eq!(admin.model_count(), 1);

        admin.unregister("User").unwrap();
        assert!(!admin.is_registered("User"));
        assert_eq!(admin.model_count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent() {
        let admin = AdminSite::new("Admin");
        let result = admin.unregister("NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_model_admin() {
        let admin = AdminSite::new("Admin");
        let model_admin = ModelAdminConfig::new("User");

        admin.register("User", model_admin).unwrap();

        let retrieved = admin.get_model_admin("User");
        assert!(retrieved.is_ok());
    }

    #[test]
    fn test_get_nonexistent_model_admin() {
        let admin = AdminSite::new("Admin");
        let result = admin.get_model_admin("NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registered_models() {
        let admin = AdminSite::new("Admin");

        admin.register("User", ModelAdminConfig::new("User")).unwrap();
        admin.register("Post", ModelAdminConfig::new("Post")).unwrap();

        let models = admin.registered_models();
        assert_eq!(models.len(), 2);
        assert!(models.contains(&"User".to_string()));
        assert!(models.contains(&"Post".to_string()));
    }

    #[test]
    fn test_clear() {
        let admin = AdminSite::new("Admin");

        admin.register("User", ModelAdminConfig::new("User")).unwrap();
        admin.register("Post", ModelAdminConfig::new("Post")).unwrap();

        assert_eq!(admin.model_count(), 2);

        admin.clear();
        assert_eq!(admin.model_count(), 0);
    }

    #[test]
    fn test_default_config() {
        let config = AdminSiteConfig::default();
        assert_eq!(config.site_title, "Admin Panel");
        assert_eq!(config.site_header, "Administration");
        assert_eq!(config.list_per_page, 100);
        assert!(config.enable_search);
        assert!(config.enable_filters);
    }
}
