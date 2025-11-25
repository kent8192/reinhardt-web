//! # Database Integration Example
//!
//! This example demonstrates database integration with Reinhardt ORM.

pub mod apps;

// Re-export commonly used types
pub use apps::users::models::User;
pub use apps::todos::models::Todo;
