//! Database maintenance statement builders
//!
//! This module provides builders for database maintenance statements:
//!
//! - VACUUM: [`VacuumStatement`]
//! - ANALYZE: [`AnalyzeStatement`]

mod analyze;
mod vacuum;

pub use analyze::*;
pub use vacuum::*;
