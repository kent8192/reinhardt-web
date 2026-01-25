//! Function DDL statement builders
//!
//! This module provides builders for function-related DDL statements:
//!
//! - CREATE FUNCTION: [`CreateFunctionStatement`]
//! - ALTER FUNCTION: [`AlterFunctionStatement`]
//! - DROP FUNCTION: [`DropFunctionStatement`]

mod alter_function;
mod create_function;
mod drop_function;

pub use alter_function::*;
pub use create_function::*;
pub use drop_function::*;
