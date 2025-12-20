//! List view Server Function
//!
//! Provides list view operations for admin models.

#[cfg(not(target_arch = "wasm32"))]
use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite, ModelAdmin};
use reinhardt_admin_types::{
	ColumnInfo, FilterChoice, FilterInfo, FilterType, ListQueryParams, ListResponse,
};
#[cfg(not(target_arch = "wasm32"))]
use reinhardt_db::orm::{Filter, FilterCondition, FilterOperator, FilterValue};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::error::MapServerFnError;

