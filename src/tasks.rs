//! Background tasks module.
//!
//! This module provides Celery-style background task execution.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt::tasks::{Task, TaskQueue};
//! ```

#[cfg(feature = "tasks")]
pub use reinhardt_tasks::*;
