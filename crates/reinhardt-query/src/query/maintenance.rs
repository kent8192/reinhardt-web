//! Database maintenance statement builders
//!
//! This module provides builders for database maintenance statements:
//!
//! - VACUUM: [`VacuumStatement`]
//! - ANALYZE: [`AnalyzeStatement`]
//! - OPTIMIZE TABLE: [`OptimizeTableStatement`] (MySQL-only)
//! - REPAIR TABLE: [`RepairTableStatement`] (MySQL-only)
//! - CHECK TABLE: [`CheckTableStatement`] (MySQL-only)

mod check_table;
mod optimize_table;
mod repair_table;

pub use check_table::*;
pub use optimize_table::*;
pub use repair_table::*;
