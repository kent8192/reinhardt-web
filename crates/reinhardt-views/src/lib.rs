//! # Reinhardt Views
//!
//! Generic views for Reinhardt framework, inspired by Django's class-based views.
//!
//! ## Features
//!
//! - **ListView**: Display a list of objects with pagination support
//! - **DetailView**: Display a single object
//! - **CreateView**: Handle object creation
//! - **UpdateView**: Handle object updates
//! - **DeleteView**: Handle object deletion
//! - **Browsable API**: HTML rendering for interactive API exploration
//! - **Interactive Docs**: Swagger UI-like documentation interface
//! - **Form Generation**: Automatic form generation for POST/PUT/PATCH methods
//! - **Syntax Highlighting**: JSON response highlighting with customizable color schemes
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_views::{ListView, DetailView, View};
//! use reinhardt_serializers::JsonSerializer;
//! use reinhardt_db::orm::{Model, QuerySet};
//! use reinhardt_core::http::{Request, Response};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct User {
//!     id: Option<i64>,
//!     username: String,
//!     email: String,
//! }
//!
//! impl Model for User {
//!     type PrimaryKey = i64;
//!     fn table_name() -> &'static str { "users" }
//!     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
//!     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
//! }
//!
//! // Create a ListView to display paginated users
//! let users = vec![
//!     User { id: Some(1), username: "alice".to_string(), email: "alice@example.com".to_string() },
//!     User { id: Some(2), username: "bob".to_string(), email: "bob@example.com".to_string() },
//! ];
//!
//! let list_view = ListView::<User, JsonSerializer<User>>::new()
//!     .with_objects(users.clone())
//!     .with_paginate_by(10)
//!     .with_ordering(vec!["-id".to_string()]);
//!
//! // Create a DetailView to display a single user
//! let detail_view = DetailView::<User, JsonSerializer<User>>::new()
//!     .with_object(users[0].clone())
//!     .with_context_object_name("user");
//!
//! // Use the views in request handlers
//! async fn handle_list(request: Request) -> Result<Response, reinhardt_exception::Error> {
//!     list_view.dispatch(request).await
//! }
//!
//! async fn handle_detail(request: Request) -> Result<Response, reinhardt_exception::Error> {
//!     detail_view.dispatch(request).await
//! }
//! ```

// Re-export from views-core
pub use reinhardt_views_core::browsable_api;
pub use reinhardt_views_core::generic;

// Re-export viewsets if the feature is enabled
#[cfg(feature = "viewsets")]
pub use reinhardt_viewsets as viewsets;

// Module declarations
mod core;
mod detail;
mod list;
mod mixins;

// Re-export public API
pub use core::{Context, View};
pub use detail::DetailView;
pub use list::ListView;
pub use mixins::{MultipleObjectMixin, SingleObjectMixin};
