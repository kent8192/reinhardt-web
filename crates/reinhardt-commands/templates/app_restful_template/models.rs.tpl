//! Models module for {{ app_name }} app
//!
//! Use the `#[user]` macro to auto-generate `BaseUser`, `FullUser`,
//! `PermissionsMixin`, and `AuthIdentity` trait implementations for user models.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt::auth::user;
//!
//! #[user]
//! #[model(db_table = "users")]
//! pub struct User {
//!     pub email: String,
//!     pub is_active: bool,
//! }
//! ```
