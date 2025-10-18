//! Audit backends
//!
//! This module provides different storage backends for audit logs.

mod file;
mod memory;

#[cfg(feature = "dynamic-database")]
mod database;

pub use file::FileAuditBackend;
pub use memory::MemoryAuditBackend;

#[cfg(feature = "dynamic-database")]
pub use database::DatabaseAuditBackend;
