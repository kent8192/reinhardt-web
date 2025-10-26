//! # Reinhardt Templates
//!
//! Template engine for Reinhardt framework using Askama (optional feature).
//!
//! ## Features
//!
//! - Variable substitution: `{{ variable }}`
//! - Control structures: `{% if %}`, `{% for %}`
//! - Template inheritance: `{% extends %}`, `{% block %}`
//! - Filters and custom filters
//! - i18n support: translation and localization
//!
//! ## Planned Features
//! TODO: Implement context processors for global context variables
//! TODO: Add custom template tags beyond filters
//! TODO: Implement automatic HTML escaping for security
//! TODO: Add include templates with `{% include %}` tag support
//! TODO: Enhance i18n support with more localization features
//! TODO: Add template debugging tools and development mode
//! TODO: Add additional template filters beyond current set
//! TODO: Enhance error messages with better diagnostics
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_templates::Template;
//! use askama::Template as AskamaTemplate;
//!
//! #[derive(Template)]
//! #[template(source = "Hello {{ name }}!", ext = "txt")]
//! struct HelloTemplate {
//!     name: String,
//! }
//!
//! let tmpl = HelloTemplate { name: "World".to_string() };
//! assert_eq!(tmpl.render().unwrap(), "Hello World!");
//! ```

pub mod advanced_filters;
pub mod custom_filters;
pub mod debug_tools;
pub mod error_reporting;
pub mod fs_loader;
pub mod i18n_filters;
pub mod static_filters;

pub use advanced_filters::{
    add, default as default_filter, filesizeformat, first, floatformat, join as join_filter, last,
    pluralize, slugify, timesince, title as title_filter, truncate as truncate_filter, urlencode,
    wordcount,
};
pub use askama::Template;
pub use custom_filters::{
    capitalize, default, join, length, ljust, lower, replace, reverse, rjust, split, striptags,
    title, trim, truncate, upper,
};
pub use debug_tools::{
    DebugPanel, TemplateContext, TemplateProfile, debug_filter, get_debug_panel,
    get_debug_panel_mut, init_debug_panel,
};
pub use error_reporting::{
    TemplateError as EnhancedTemplateError, TemplateErrorContext, suggest_similar,
};
pub use fs_loader::FileSystemTemplateLoader;
pub use reinhardt_exception::Error as TemplateError;
pub type TemplateResult<T> = reinhardt_exception::Result<T>;
pub use i18n_filters::{
    blocktrans, blocktrans_plural, get_current_language, localize_date_filter,
    localize_number_filter, trans, trans_with_context,
};
pub use static_filters::{StaticConfig, init_static_config, static_filter, static_path_join};

use reinhardt_exception::{Error, Result};
use std::collections::HashMap;

/// Template loader for managing multiple templates
pub struct TemplateLoader {
    templates: HashMap<String, Box<dyn Fn() -> String + Send + Sync>>,
}

impl TemplateLoader {
    /// Creates a new TemplateLoader instance
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateLoader;
    ///
    /// let loader = TemplateLoader::new();
    // Loader is ready to register templates
    /// ```
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Register a template rendering function
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateLoader;
    ///
    /// let mut loader = TemplateLoader::new();
    /// loader.register("hello", || "Hello World!".to_string());
    ///
    /// let result = loader.render("hello").unwrap();
    /// assert_eq!(result, "Hello World!");
    /// ```
    pub fn register<F>(&mut self, name: impl Into<String>, render_fn: F)
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.templates.insert(name.into(), Box::new(render_fn));
    }

    /// Render a registered template by name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateLoader;
    ///
    /// let mut loader = TemplateLoader::new();
    /// loader.register("greeting", || "Hello, {{ name }}!".to_string());
    ///
    /// let result = loader.render("greeting").unwrap();
    /// assert_eq!(result, "Hello, {{ name }}!");
    ///
    // Template not found
    /// let error = loader.render("nonexistent").unwrap_err();
    /// assert!(matches!(error, reinhardt_templates::TemplateError::TemplateNotFound(_)));
    /// ```
    pub fn render(&self, name: &str) -> Result<String> {
        self.templates
            .get(name)
            .map(|f| f())
            .ok_or_else(|| Error::TemplateNotFound(name.to_string()))
    }
}

// Manually implement Send and Sync for TemplateLoader
unsafe impl Send for TemplateLoader {}
unsafe impl Sync for TemplateLoader {}

impl Default for TemplateLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Type-safe template loading (compile-time checked)
// ============================================================================

/// Trait for templates that can be loaded at compile time
///
/// Implement this trait for each template in your application.
/// The compiler will ensure that only valid templates can be loaded.
///
/// # Example
///
/// ```rust
/// use reinhardt_templates::TemplateId;
///
/// pub struct UserListTemplate;
/// impl TemplateId for UserListTemplate {
///     const NAME: &'static str = "user_list.html";
/// }
/// ```
pub trait TemplateId {
    /// The unique name for this template
    const NAME: &'static str;
}

impl TemplateLoader {
    /// Type-safe render method that takes a TemplateId type parameter
    ///
    /// This method ensures at compile time that only valid template types
    /// can be used.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_templates::{TemplateLoader, TemplateId};
    ///
    /// pub struct HomeTemplate;
    /// impl TemplateId for HomeTemplate {
    ///     const NAME: &'static str = "home.html";
    /// }
    ///
    /// let mut loader = TemplateLoader::new();
    /// loader.register_typed::<HomeTemplate, _>(|| "<h1>Home Page</h1>".to_string());
    ///
    /// let html = loader.render_typed::<HomeTemplate>().unwrap();
    /// assert_eq!(html, "<h1>Home Page</h1>");
    /// ```
    pub fn render_typed<T: TemplateId>(&self) -> Result<String> {
        self.render(T::NAME)
    }

    /// Type-safe registration method
    ///
    /// Register a template rendering function with compile-time type checking.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_templates::{TemplateLoader, TemplateId};
    ///
    /// pub struct HomeTemplate;
    /// impl TemplateId for HomeTemplate {
    ///     const NAME: &'static str = "home.html";
    /// }
    ///
    /// let mut loader = TemplateLoader::new();
    /// loader.register_typed::<HomeTemplate, _>(|| "Hello, World!".to_string());
    /// ```
    pub fn register_typed<T: TemplateId, F>(&mut self, render_fn: F)
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.register(T::NAME, render_fn);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;
    use askama::Template as AskamaTemplate;

    #[derive(AskamaTemplate)]
    #[template(source = "Hello {{ name }}!", ext = "txt")]
    struct HelloTemplate {
        name: String,
    }

    #[derive(AskamaTemplate)]
    #[template(source = "{{ greeting }} {{ name }}!", ext = "txt")]
    struct GreetingTemplate {
        greeting: String,
        name: String,
    }

    #[test]
    fn test_simple_variable_substitution() {
        let tmpl = HelloTemplate {
            name: "World".to_string(),
        };
        assert_eq!(tmpl.render().unwrap(), "Hello World!");
    }

    #[test]
    fn test_multiple_variables() {
        let tmpl = GreetingTemplate {
            greeting: "Hello".to_string(),
            name: "Rust".to_string(),
        };
        assert_eq!(tmpl.render().unwrap(), "Hello Rust!");
    }

    #[test]
    fn test_template_loader() {
        let mut loader = TemplateLoader::new();
        loader.register("hello", || {
            let tmpl = HelloTemplate {
                name: "World".to_string(),
            };
            tmpl.render().unwrap()
        });

        assert_eq!(loader.render("hello").unwrap(), "Hello World!");
    }

    // Type-safe template tests
    struct HomeTemplateId;
    impl TemplateId for HomeTemplateId {
        const NAME: &'static str = "home.html";
    }

    struct UserListTemplateId;
    impl TemplateId for UserListTemplateId {
        const NAME: &'static str = "user_list.html";
    }

    #[test]
    fn test_typed_template_registration() {
        let mut loader = TemplateLoader::new();
        loader.register_typed::<HomeTemplateId, _>(|| "Home Page".to_string());

        assert_eq!(loader.render("home.html").unwrap(), "Home Page");
    }

    #[test]
    fn test_typed_template_render() {
        let mut loader = TemplateLoader::new();
        loader.register_typed::<UserListTemplateId, _>(|| "User List".to_string());

        let result = loader.render_typed::<UserListTemplateId>().unwrap();
        assert_eq!(result, "User List");
    }

    #[test]
    fn test_typed_template_not_found() {
        let loader = TemplateLoader::new();

        let result = loader.render_typed::<HomeTemplateId>();
        assert!(result.is_err());

        if let Err(TemplateError::TemplateNotFound(name)) = result {
            assert_eq!(name, "home.html");
        }
    }

    #[test]
    fn test_templates_typed_and_regular_mixed() {
        let mut loader = TemplateLoader::new();

        // Register using typed method
        loader.register_typed::<HomeTemplateId, _>(|| "Home".to_string());

        // Register using regular method
        loader.register("about.html", || "About".to_string());

        // Can use both methods to access
        assert_eq!(loader.render_typed::<HomeTemplateId>().unwrap(), "Home");
        assert_eq!(loader.render("about.html").unwrap(), "About");
    }
}
