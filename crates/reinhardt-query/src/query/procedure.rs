//! Procedure DDL statement builders
//!
//! This module provides builders for procedure-related DDL statements:
//!
//! - [`CreateProcedureStatement`]: CREATE PROCEDURE statement
//! - [`AlterProcedureStatement`]: ALTER PROCEDURE statement
//! - [`DropProcedureStatement`]: DROP PROCEDURE statement

pub mod alter_procedure;
pub mod create_procedure;
pub mod drop_procedure;

pub use alter_procedure::AlterProcedureStatement;
pub use create_procedure::CreateProcedureStatement;
pub use drop_procedure::DropProcedureStatement;
