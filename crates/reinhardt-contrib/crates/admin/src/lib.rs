//! # Reinhardt Admin
//!
//! Django-style auto-generated admin interface for Reinhardt framework.
//!
//! ## Overview
//!
//! Reinhardt Admin provides a powerful, customizable administration panel for managing
//! database models through a web interface. It's inspired by Django's admin panel but
//! built with Rust's type safety and performance in mind.
//!
//! ## Features
//!
//! - ✅ Auto-generated CRUD interfaces from models
//! - ✅ List views with filtering, searching, and sorting
//! - ✅ Detail/Edit forms with validation
//! - ✅ Batch actions (delete, custom actions)
//! - ✅ Permission-based access control
//! - ✅ Customizable templates
//! - ✅ Inline editing for related models
//!
//! ## Quick Start
//!
//! ### 1. Define Your Model
//!
//! ```rust,ignore
//! use reinhardt_orm::Model;
//!
//! #[derive(Model)]
//! #[reinhardt(table_name = "users")]
//! pub struct User {
//!     #[reinhardt(primary_key)]
//!     pub id: i64,
//!     pub username: String,
//!     pub email: String,
//!     pub is_active: bool,
//! }
//! ```
//!
//! ### 2. Register with Admin
//!
//! ```rust,ignore
//! use reinhardt_admin::{AdminSite, ModelAdmin};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut admin = AdminSite::new("My Admin");
//!
//!     // Simple registration
//!     admin.register::<User>(UserAdmin::default()).await;
//!
//!     // Start admin server
//!     admin.serve("127.0.0.1:8001").await.unwrap();
//! }
//! ```
//!
//! ### 3. Customize the Admin
//!
//! ```rust,ignore
//! use reinhardt_admin::ModelAdmin;
//!
//! struct UserAdmin {
//!     list_display: Vec<String>,
//!     list_filter: Vec<String>,
//!     search_fields: Vec<String>,
//! }
//!
//! impl Default for UserAdmin {
//!     fn default() -> Self {
//!         Self {
//!             list_display: vec!["username".to_string(), "email".to_string(), "is_active".to_string()],
//!             list_filter: vec!["is_active".to_string()],
//!             search_fields: vec!["username".to_string(), "email".to_string()],
//!         }
//!     }
//! }
//! ```
//!
//! ## Comparison with Django Admin
//!
//! | Feature | Django Admin | Reinhardt Admin |
//! |---------|--------------|-----------------|
//! | Auto CRUD | ✅ | ✅ |
//! | List Filters | ✅ | ✅ |
//! | Search | ✅ | ✅ |
//! | Inline Editing | ✅ | ✅ |
//! | Custom Actions | ✅ | ✅ |
//! | Type Safety | ⚠️ Runtime | ✅ Compile-time |
//! | Performance | ⚠️ Python | ✅ Native Rust |
//! | Async Support | ⚠️ Limited | ✅ Full async |
//!
//! ## Security
//!
//! Admin panel should **only** be used for trusted internal staff, not for general users.
//! Always ensure proper authentication and authorization.
//!
//! ## Planned Features
//!
//! - [ ] Advanced filtering (date range, number range)
//! - [ ] Bulk import/export (CSV, JSON)
//! - [ ] Audit logs
//! - [ ] Dashboard widgets
//! - [ ] Custom views registration
//! - [ ] Rich text editor integration
//! - [ ] Image upload preview
//! - [ ] Drag & drop reordering

// Core modules
pub mod site;
pub mod model_admin;
pub mod actions;
pub mod filters;
pub mod views;
pub mod forms;

// Re-exports
pub use site::AdminSite;
pub use model_admin::{ModelAdmin, ModelAdminConfig};
pub use actions::{AdminAction, ActionResult};
pub use filters::{ListFilter, FilterSpec};
pub use views::{ListView, DetailView, CreateView, UpdateView, DeleteView};

/// Admin panel error types
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    /// Model not registered with admin
    #[error("Model '{0}' is not registered with admin")]
    ModelNotRegistered(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid action
    #[error("Invalid action: {0}")]
    InvalidAction(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type AdminResult<T> = Result<T, AdminError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_error_display() {
        let err = AdminError::ModelNotRegistered("User".to_string());
        assert_eq!(err.to_string(), "Model 'User' is not registered with admin");

        let err = AdminError::PermissionDenied("Not an admin user".to_string());
        assert_eq!(err.to_string(), "Permission denied: Not an admin user");
    }
}
