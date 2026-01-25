//! Type definitions for identifiers, column references, and table references.
//!
//! This module provides the core type system for identifying SQL entities:
//!
//! - [`Iden`]: Trait for SQL identifiers (tables, columns, schemas)
//! - [`IdenStatic`]: Marker trait for compile-time identifiers
//! - [`Alias`]: Dynamic identifier for runtime-determined names
//! - [`DynIden`]: Type-erased identifier for heterogeneous collections
//! - [`ColumnRef`]: Reference to a column (simple, table-qualified, or schema-qualified)
//! - [`TableRef`]: Reference to a table (simple, schema-qualified, aliased, or subquery)
//! - [`IntoIden`]: Conversion trait for identifier types
//! - [`IntoColumnRef`]: Conversion trait for column references
//! - [`IntoTableRef`]: Conversion trait for table references

mod alias;
mod column_ref;
pub mod comment;
pub mod database;
mod ddl;
mod iden;
mod join;
mod operators;
mod order;
pub mod schema;
pub mod sequence;
mod table_ref;
mod trigger;
mod window;
pub mod zone;

pub use alias::Alias;
pub use column_ref::{ColumnRef, IntoColumnRef};
pub use comment::CommentTarget;
pub use database::DatabaseOperation;
pub use ddl::{ColumnDef, ColumnType, ForeignKeyAction, IndexDef, TableConstraint};
pub use iden::{DynIden, Iden, IdenStatic, IntoIden, SeaRc};
pub use join::{ColumnPair, ColumnSpec, JoinExpr, JoinOn, JoinType};
pub use operators::{BinOper, LogicalChainOper, PgBinOper, SubQueryOper, UnOper};
pub use order::{NullOrdering, Order, OrderExpr, OrderExprKind};
pub use schema::SchemaDef;
pub use sequence::{OwnedBy, SequenceDef, SequenceOption};
pub use table_ref::{IntoTableRef, TableRef};
pub use trigger::*;
pub use window::{Frame, FrameClause, FrameType, WindowStatement};
pub use zone::ZoneConfig;

#[cfg(test)]
mod tests;
