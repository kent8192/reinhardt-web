//! reinhardt-pages integration tests
//!
//! This module contains comprehensive integration tests for the reinhardt-pages crate,
//! including Server Functions, CSRF protection, and API model CRUD operations.

#[path = "pages/fixtures.rs"]
pub mod fixtures;

#[path = "pages/server_fn_execution_integration.rs"]
pub mod server_fn_execution_integration;

#[path = "pages/csrf_protection_integration.rs"]
pub mod csrf_protection_integration;

#[path = "pages/api_model_crud_integration.rs"]
pub mod api_model_crud_integration;
