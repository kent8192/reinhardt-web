//! Function DDL statement builders
//!
//! This module provides builders for function-related DDL statements:
//!
//! - [`CreateFunctionStatement`]: CREATE FUNCTION statement
//! - [`AlterFunctionStatement`]: ALTER FUNCTION statement
//! - [`DropFunctionStatement`]: DROP FUNCTION statement

pub mod alter_function;
pub mod create_function;
pub mod drop_function;

pub use alter_function::AlterFunctionStatement;
pub use create_function::CreateFunctionStatement;
pub use drop_function::DropFunctionStatement;
