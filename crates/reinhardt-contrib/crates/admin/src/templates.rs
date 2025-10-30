//! Template rendering for admin interface
//!
//! This module provides template rendering using the Tera template engine.
//! Previously used Askama, but migrated to Tera to resolve:
//! - HashMap.get() type inference issues
//! - Option<String> HtmlSafe handling
//! - Better support for dynamic content and runtime template loading
//!
//! The Tera templates use Jinja2-style syntax, compatible with Django templates.

use crate::{AdminError, AdminResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tera::Tera;

#[allow(unused_imports)]
use crate::filters;

// NOTE: Tera provides built-in filters similar to Django/Jinja2
// Custom filters can be registered on the Tera instance if needed
// See: https://tera.netlify.app/docs/#filters

/// Admin template context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminContext {
    /// Site title
    pub site_title: String,
    /// Site header
    pub site_header: String,
    /// Current user
    pub user: Option<UserContext>,
    /// Available apps
    pub available_apps: Vec<AppContext>,
    /// Extra context data
    pub extra: HashMap<String, serde_json::Value>,
}

impl AdminContext {
    /// Create a new admin context
    pub fn new(site_title: impl Into<String>) -> Self {
        Self {
            site_title: site_title.into(),
            site_header: "Administration".to_string(),
            user: None,
            available_apps: Vec::new(),
            extra: HashMap::new(),
        }
    }

    /// Set site header
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.site_header = header.into();
        self
    }

    /// Set current user
    pub fn with_user(mut self, user: UserContext) -> Self {
        self.user = Some(user);
        self
    }

    /// Add an app
    pub fn add_app(&mut self, app: AppContext) {
        self.available_apps.push(app);
    }

    /// Add extra context data
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

impl Default for AdminContext {
    fn default() -> Self {
        Self::new("Reinhardt Admin")
    }
}

/// User context for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub username: String,
    pub email: Option<String>,
    pub is_staff: bool,
    pub is_superuser: bool,
}

/// App context for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppContext {
    pub name: String,
    pub label: String,
    pub models: Vec<ModelContext>,
}

/// Model context for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContext {
    pub name: String,
    pub label: String,
    pub url: String,
    pub add_url: Option<String>,
}

/// List view template context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewContext {
    /// Base admin context
    pub admin: AdminContext,
    /// Model name
    pub model_name: String,
    /// Model verbose name
    pub model_verbose_name: String,
    /// List of items
    pub items: Vec<HashMap<String, serde_json::Value>>,
    /// Fields to display
    pub list_display: Vec<String>,
    /// Field labels
    pub field_labels: HashMap<String, String>,
    /// Available filters
    pub filters: Vec<FilterContext>,
    /// Search query
    pub search_query: Option<String>,
    /// Pagination info
    pub pagination: PaginationContext,
    /// Available actions
    pub actions: Vec<ActionContext>,
}

impl ListViewContext {
    /// Create a new list view context
    pub fn new(
        model_name: impl Into<String>,
        items: Vec<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            admin: AdminContext::default(),
            model_name: model_name.into(),
            model_verbose_name: "Items".to_string(),
            items,
            list_display: Vec::new(),
            field_labels: HashMap::new(),
            filters: Vec::new(),
            search_query: None,
            pagination: PaginationContext::default(),
            actions: Vec::new(),
        }
    }
}

/// Filter context for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterContext {
    pub title: String,
    pub choices: Vec<FilterChoice>,
}

/// Filter choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterChoice {
    pub label: String,
    pub url: String,
    pub selected: bool,
}

/// Pagination context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationContext {
    pub page: usize,
    pub total_pages: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub has_previous: bool,
    pub has_next: bool,
    pub previous_url: Option<String>,
    pub next_url: Option<String>,
}

impl Default for PaginationContext {
    fn default() -> Self {
        Self {
            page: 1,
            total_pages: 1,
            page_size: 100,
            total_count: 0,
            has_previous: false,
            has_next: false,
            previous_url: None,
            next_url: None,
        }
    }
}

/// Action context for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub name: String,
    pub label: String,
    pub description: String,
}

/// Form view template context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormViewContext {
    /// Base admin context
    pub admin: AdminContext,
    /// Model name
    pub model_name: String,
    /// Form title
    pub title: String,
    /// Form fields
    pub fields: Vec<FormFieldContext>,
    /// Inline formsets
    pub inlines: Vec<InlineFormsetContext>,
    /// Object ID (for edit)
    pub object_id: Option<String>,
    /// Form errors
    pub errors: Vec<String>,
}

/// Form field context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldContext {
    pub name: String,
    pub label: String,
    pub widget_html: String,
    pub help_text: Option<String>,
    pub errors: Vec<String>,
    pub is_readonly: bool,
    pub is_required: bool,
}

/// Inline formset context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineFormsetContext {
    pub model_name: String,
    pub verbose_name: String,
    pub forms: Vec<FormViewContext>,
}

/// Delete confirmation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteConfirmationContext {
    /// Base admin context
    pub admin: AdminContext,
    /// Model name
    pub model_name: String,
    /// Object representation
    pub object_repr: String,
    /// Related objects that will be deleted
    pub related_objects: Vec<RelatedObjectContext>,
    /// Total count
    pub total_count: usize,
}

/// Related object context for delete confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedObjectContext {
    pub model_name: String,
    pub count: usize,
    pub items: Vec<String>,
}

/// Dashboard context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardContext {
    /// Base admin context
    pub admin: AdminContext,
    /// Widget HTML content
    pub widgets: Vec<WidgetContext>,
    /// Recent actions
    pub recent_actions: Vec<RecentActionContext>,
}

/// Widget context for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetContext {
    pub title: String,
    pub content_html: String,
    pub css_class: Option<String>,
}

/// Recent action context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActionContext {
    pub action: String,
    pub model_name: String,
    pub object_repr: String,
    pub user: String,
    pub timestamp: String,
}

// ============================================================================
// Template Renderer - Using Tera Template Engine
// ============================================================================
// Note: Previously used Askama with struct-based templates, but migrated to Tera
// to resolve HashMap.get() type inference and Option<String> HtmlSafe issues.
// All template rendering is now done through AdminTemplateRenderer methods.

/// Template renderer
pub struct AdminTemplateRenderer {
    template_dir: String,
    tera: Arc<Tera>,
}

impl AdminTemplateRenderer {
    /// Create a new template renderer
    pub fn new(template_dir: impl Into<String>) -> Self {
        let template_dir = template_dir.into();
        
        // Initialize Tera with templates from the admin templates directory
        let template_path = format!("{}/**/*.tpl", template_dir);
        let tera = match Tera::new(&template_path) {
            Ok(t) => Arc::new(t),
            Err(e) => {
                // If templates can't be loaded, create an empty Tera instance
                eprintln!("Warning: Failed to load templates from {}: {}", template_path, e);
                Arc::new(Tera::default())
            }
        };
        
        Self {
            template_dir,
            tera,
        }
    }
    
    /// Create a new template renderer with a custom Tera instance
    pub fn with_tera(template_dir: impl Into<String>, tera: Tera) -> Self {
        Self {
            template_dir: template_dir.into(),
            tera: Arc::new(tera),
        }
    }

    /// Render list view using Tera template engine
    /// This method uses Tera instead of Askama to avoid HashMap.get() type inference issues
    pub fn render_list(&self, context: &ListViewContext) -> AdminResult<String> {
        let mut tera_context = tera::Context::new();
        
        // Base context
        tera_context.insert("site_title", &context.admin.site_title);
        tera_context.insert("site_header", &context.admin.site_header);
        tera_context.insert("user", &context.admin.user);
        tera_context.insert("available_apps", &context.admin.available_apps);
        
        // List view specific
        tera_context.insert("model_name", &context.model_name);
        tera_context.insert("model_verbose_name", &context.model_verbose_name);
        tera_context.insert("items", &context.items);
        tera_context.insert("list_display", &context.list_display);
        tera_context.insert("field_labels", &context.field_labels);
        tera_context.insert("filters", &context.filters);
        tera_context.insert("search_query", &context.search_query);
        tera_context.insert("pagination", &context.pagination);
        tera_context.insert("actions", &context.actions);
        
        self.tera
            .render("list.tpl", &tera_context)
            .map_err(|e| AdminError::TemplateError(format!("Failed to render list template: {}", e)))
    }

    /// Render form view using Tera template engine
    pub fn render_form(&self, context: &FormViewContext) -> AdminResult<String> {
        let mut tera_context = tera::Context::new();
        
        // Base context
        tera_context.insert("site_title", &context.admin.site_title);
        tera_context.insert("site_header", &context.admin.site_header);
        tera_context.insert("user", &context.admin.user);
        tera_context.insert("available_apps", &context.admin.available_apps);
        
        // Form view specific
        tera_context.insert("model_name", &context.model_name);
        tera_context.insert("title", &context.title);
        tera_context.insert("fields", &context.fields);
        tera_context.insert("inlines", &context.inlines);
        tera_context.insert("object_id", &context.object_id);
        tera_context.insert("errors", &context.errors);
        
        self.tera
            .render("form.tpl", &tera_context)
            .map_err(|e| AdminError::TemplateError(format!("Failed to render form template: {}", e)))
    }

    /// Render delete confirmation using Tera template engine
    pub fn render_delete_confirmation(
        &self,
        context: &DeleteConfirmationContext,
    ) -> AdminResult<String> {
        let mut tera_context = tera::Context::new();
        
        // Base context
        tera_context.insert("site_title", &context.admin.site_title);
        tera_context.insert("site_header", &context.admin.site_header);
        tera_context.insert("user", &context.admin.user);
        tera_context.insert("available_apps", &context.admin.available_apps);
        
        // Delete confirmation specific
        tera_context.insert("model_name", &context.model_name);
        tera_context.insert("object_repr", &context.object_repr);
        tera_context.insert("related_objects", &context.related_objects);
        tera_context.insert("total_count", &context.total_count);
        
        self.tera
            .render("delete_confirmation.tpl", &tera_context)
            .map_err(|e| AdminError::TemplateError(format!("Failed to render delete confirmation template: {}", e)))
    }

    /// Render dashboard using Tera template engine
    /// This method uses Tera instead of Askama to avoid Option<String> HtmlSafe handling issues
    pub fn render_dashboard(&self, context: &DashboardContext) -> AdminResult<String> {
        let mut tera_context = tera::Context::new();
        
        // Base context
        tera_context.insert("site_title", &context.admin.site_title);
        tera_context.insert("site_header", &context.admin.site_header);
        tera_context.insert("user", &context.admin.user);
        tera_context.insert("available_apps", &context.admin.available_apps);
        
        // Dashboard specific
        tera_context.insert("widgets", &context.widgets);
        tera_context.insert("recent_actions", &context.recent_actions);
        
        self.tera
            .render("dashboard.tpl", &tera_context)
            .map_err(|e| AdminError::TemplateError(format!("Failed to render dashboard template: {}", e)))
    }

}

impl Default for AdminTemplateRenderer {
    fn default() -> Self {
        Self::new("crates/reinhardt-contrib/crates/admin/templates")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_context_new() {
        let ctx = AdminContext::new("My Admin");
        assert_eq!(ctx.site_title, "My Admin");
        assert_eq!(ctx.site_header, "Administration");
        assert!(ctx.user.is_none());
    }

    #[test]
    fn test_list_view_context() {
        let mut item = HashMap::new();
        item.insert("id".to_string(), serde_json::json!("1"));
        item.insert("name".to_string(), serde_json::json!("Test"));

        let ctx = ListViewContext::new("User", vec![item]);
        assert_eq!(ctx.model_name, "User");
        assert_eq!(ctx.items.len(), 1);
    }

    #[test]
    fn test_pagination_context_default() {
        let pag = PaginationContext::default();
        assert_eq!(pag.page, 1);
        assert_eq!(pag.page_size, 100);
        assert!(!pag.has_previous);
        assert!(!pag.has_next);
    }

    #[test]
    fn test_template_renderer_list() {
        let renderer = AdminTemplateRenderer::default();

        let mut item = HashMap::new();
        item.insert("id".to_string(), serde_json::json!("1"));
        item.insert("username".to_string(), serde_json::json!("alice"));

        let mut ctx = ListViewContext::new("User", vec![item]);
        ctx.list_display = vec!["id".to_string(), "username".to_string()];

        let html = renderer.render_list(&ctx).unwrap();
        assert!(html.contains("User"));
        assert!(html.contains("alice"));
    }
}
