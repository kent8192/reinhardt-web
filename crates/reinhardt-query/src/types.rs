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
mod iden;
mod join;
mod operators;
mod order;
mod table_ref;
mod window;

pub use alias::Alias;
pub use column_ref::{ColumnRef, IntoColumnRef};
pub use iden::{DynIden, Iden, IdenStatic, IntoIden, SeaRc};
pub use join::{ColumnPair, ColumnSpec, JoinExpr, JoinOn, JoinType};
pub use operators::{BinOper, LogicalChainOper, SubQueryOper, UnOper};
pub use order::{NullOrdering, Order, OrderExpr, OrderExprKind};
pub use table_ref::{IntoTableRef, TableRef};
pub use window::{Frame, FrameClause, FrameType, WindowStatement};

#[cfg(test)]
mod tests;
