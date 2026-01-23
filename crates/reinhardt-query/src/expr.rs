//! Expression system for SQL queries.
//!
//! This module provides types and traits for building SQL expressions:
//!
//! - [`SimpleExpr`]: The core expression AST
//! - [`Expr`]: Builder for creating expressions
//! - [`ExprTrait`]: Trait providing expression operations
//! - [`Condition`] and [`Cond`]: Condition building for WHERE/HAVING clauses
//! - [`CaseStatement`]: CASE WHEN expressions

mod condition;
mod expr;
mod expr_trait;
mod simple_expr;

pub use condition::{
	Cond, Condition, ConditionExpression, ConditionHolder, ConditionType, IntoCondition,
};
pub use expr::{CaseExprBuilder, Expr};
pub use expr_trait::ExprTrait;
pub use simple_expr::{CaseStatement, Keyword, SimpleExpr, SubQueryOper};

#[cfg(test)]
mod tests;
