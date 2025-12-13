//! # reinhardt-admin
//!
//! Admin functionality for Reinhardt framework.
//!
//! This crate provides two main components:
//! - **Panel**: Django-style web admin panel for managing models
//! - **CLI**: Command-line tool for project management
//!
//! ## Features
//!
//! - `panel` (default): Web admin panel
//! - `cli`: Command-line interface
//! - `all`: All admin functionality
//!
//! ## Examples
//!
//! ### Using the admin panel
//!
//! ```rust,no_run
//! # use reinhardt_admin::panel::AdminSite;
//! let site = AdminSite::new("Admin");
//! // Register models...
//! ```
//!
//! ### Using the CLI
//!
//! ```bash
//! reinhardt-admin startproject my_project
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "panel")]
#[cfg_attr(docsrs, doc(cfg(feature = "panel")))]
pub use reinhardt_panel as panel;
