//! Custom type DDL statement builders (PostgreSQL, CockroachDB)
//!
//! This module provides builders for custom type-related DDL statements:
//!
//! - [`CreateTypeStatement`]: CREATE TYPE statement
//! - [`AlterTypeStatement`]: ALTER TYPE statement
//! - [`DropTypeStatement`]: DROP TYPE statement
//!
//! **Note**: Custom types are only supported by PostgreSQL and CockroachDB.
//! MySQL and SQLite will panic with a helpful error message.

pub mod alter_type;
pub mod create_type;
pub mod drop_type;

pub use alter_type::AlterTypeStatement;
pub use create_type::CreateTypeStatement;
pub use drop_type::DropTypeStatement;
