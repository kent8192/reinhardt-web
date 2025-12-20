//! Detail view Server Function
//!
//! Provides detail view operations for admin models.

#[cfg(not(target_arch = "wasm32"))]
use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::DetailResponse;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::error::MapServerFnError;
