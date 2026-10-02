//! DDL-safe expressions for schema definitions.

use crate::types::{ColumnType, DynIden, IntoIden};
use crate::value::{IntoValue, Value};

/// DDL-safe expression subset for generated columns.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SchemaExpr {
	/// Column reference by identifier.
	Column(DynIden),
	/// Inline literal value.
	Value(Value),
	/// Binary arithmetic expression.
	Binary {
		/// Left operand.
		left: Box<SchemaExpr>,
		/// Binary operator.
		op: SchemaBinOper,
		/// Right operand.
		right: Box<SchemaExpr>,
	},
	/// Backend-rendered function call.
	Function {
		/// Function kind.
		func: SchemaFunc,
		/// Function arguments.
		args: Vec<SchemaExpr>,
	},
	/// SQL cast expression.
	Cast {
		/// Expression to cast.
		expr: Box<SchemaExpr>,
		/// Target column type.
		ty: ColumnType,
	},
}

impl PartialEq for SchemaExpr {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Column(left), Self::Column(right)) => left.to_string() == right.to_string(),
			(Self::Value(left), Self::Value(right)) => left == right,
			(
				Self::Binary {
					left: left_expr,
					op: left_op,
					right: left_right,
				},
				Self::Binary {
					left: right_expr,
					op: right_op,
					right: right_right,
				},
			) => left_expr == right_expr && left_op == right_op && left_right == right_right,
			(
				Self::Function {
					func: left_func,
					args: left_args,
				},
				Self::Function {
					func: right_func,
					args: right_args,
				},
			) => left_func == right_func && left_args == right_args,
			(
				Self::Cast {
					expr: left_expr,
					ty: left_ty,
				},
				Self::Cast {
					expr: right_expr,
					ty: right_ty,
				},
			) => left_expr == right_expr && left_ty == right_ty,
			_ => false,
		}
	}
}

/// Binary operators allowed in schema expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaBinOper {
	/// Addition.
	Add,
	/// Subtraction.
	Sub,
	/// Multiplication.
	Mul,
	/// Division.
	Div,
}

impl SchemaBinOper {
	/// Return the SQL operator token.
	#[must_use]
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Add => "+",
			Self::Sub => "-",
			Self::Mul => "*",
			Self::Div => "/",
		}
	}
}

/// Functions allowed in schema expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaFunc {
	/// String concatenation.
	Concat,
	/// First non-null value.
	Coalesce,
}

/// Generated-column storage mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GeneratedStorage {
	/// Persist the generated value.
	Stored,
	/// Compute the generated value on read.
	Virtual,
}

impl GeneratedStorage {
	/// Return the SQL storage keyword.
	#[must_use]
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Stored => "STORED",
			Self::Virtual => "VIRTUAL",
		}
	}
}

/// Generated-column expression metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedColumn {
	/// Typed generated expression.
	pub expr: Option<SchemaExpr>,
	/// Explicit backend-specific raw SQL body.
	pub raw_sql: Option<String>,
	/// Generated-column storage mode.
	pub storage: GeneratedStorage,
}

impl SchemaExpr {
	/// Create a column reference.
	#[must_use]
	pub fn col(name: impl IntoIden) -> Self {
		Self::Column(name.into_iden())
	}

	/// Create an inline literal value.
	#[must_use]
	pub fn val(value: impl IntoValue) -> Self {
		Self::Value(value.into_value())
	}

	/// Create a binary arithmetic expression.
	#[must_use]
	pub fn binary(self, op: SchemaBinOper, right: SchemaExpr) -> Self {
		Self::Binary {
			left: Box::new(self),
			op,
			right: Box::new(right),
		}
	}

	/// Create a backend-rendered concat expression.
	#[must_use]
	pub fn concat<I>(items: I) -> Self
	where
		I: IntoIterator<Item = SchemaExpr>,
	{
		Self::Function {
			func: SchemaFunc::Concat,
			args: items.into_iter().collect(),
		}
	}

	/// Create a COALESCE expression.
	#[must_use]
	pub fn coalesce<I>(items: I) -> Self
	where
		I: IntoIterator<Item = SchemaExpr>,
	{
		let args = items.into_iter().collect::<Vec<_>>();
		assert!(
			!args.is_empty(),
			"SchemaExpr::coalesce requires at least one argument"
		);
		Self::Function {
			func: SchemaFunc::Coalesce,
			args,
		}
	}

	/// Create a CAST expression.
	#[must_use]
	pub fn cast(self, ty: ColumnType) -> Self {
		Self::Cast {
			expr: Box::new(self),
			ty,
		}
	}
}

impl GeneratedColumn {
	/// Create typed generated-column metadata.
	#[must_use]
	pub fn typed(expr: SchemaExpr, storage: GeneratedStorage) -> Self {
		Self {
			expr: Some(expr),
			raw_sql: None,
			storage,
		}
	}

	/// Create explicit raw-SQL generated-column metadata.
	#[must_use]
	pub fn raw_sql(sql: impl Into<String>, storage: GeneratedStorage) -> Self {
		Self {
			expr: None,
			raw_sql: Some(sql.into()),
			storage,
		}
	}

	/// Validate that exactly one generated-column body is present.
	pub fn validate(&self) -> Result<(), &'static str> {
		match (self.expr.is_some(), self.raw_sql.is_some()) {
			(true, false) | (false, true) => Ok(()),
			_ => Err("generated columns require exactly one typed expression or raw SQL body"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn concat_stores_ordered_arguments() {
		let expr = SchemaExpr::concat([
			SchemaExpr::col("first_name"),
			SchemaExpr::val(" "),
			SchemaExpr::col("last_name"),
		]);

		match expr {
			SchemaExpr::Function { func, args } => {
				assert_eq!(func, SchemaFunc::Concat);
				assert_eq!(args.len(), 3);
				assert_eq!(args[0], SchemaExpr::col("first_name"));
				assert_eq!(args[1], SchemaExpr::val(" "));
				assert_eq!(args[2], SchemaExpr::col("last_name"));
			}
			other => panic!("expected concat function, got {other:?}"),
		}
	}

	#[test]
	#[should_panic(expected = "SchemaExpr::coalesce requires at least one argument")]
	fn coalesce_rejects_empty_arguments() {
		let _ = SchemaExpr::coalesce(std::iter::empty::<SchemaExpr>());
	}

	#[test]
	fn generated_column_requires_exactly_one_body() {
		let typed = GeneratedColumn::typed(SchemaExpr::col("name"), GeneratedStorage::Stored);
		assert!(typed.validate().is_ok());

		let raw = GeneratedColumn::raw_sql("LOWER(name)", GeneratedStorage::Stored);
		assert!(raw.validate().is_ok());

		let invalid = GeneratedColumn {
			expr: None,
			raw_sql: None,
			storage: GeneratedStorage::Stored,
		};
		assert_eq!(
			invalid.validate(),
			Err("generated columns require exactly one typed expression or raw SQL body")
		);
	}

	#[test]
	fn column_expression_equality_compares_identifier_names() {
		assert_eq!(SchemaExpr::col("first_name"), SchemaExpr::col("first_name"));
		assert_ne!(SchemaExpr::col("first_name"), SchemaExpr::col("last_name"));
	}
}
