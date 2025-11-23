//! Application registry for examples-database-integration
//!
//! This file maintains the list of installed apps.

// Re-export apps from the apps directory
#[path = "apps/todos/lib.rs"]
pub mod todos;

pub use todos::TodosConfig;
