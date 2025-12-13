//! Views module.
//!
//! This module provides class-based views and viewsets.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt::views::{ListView, DetailView};
//! # #[cfg(feature = "viewsets")]
//! use reinhardt::views::viewsets::ModelViewSet;
//!
//! # #[derive(Clone)]
//! # struct User;
//! // Create views for User model
//! # let _list_view = ListView::<User>::new();
//! # let _detail_view = DetailView::<User>::new();
//! ```

pub use reinhardt_views::*;
