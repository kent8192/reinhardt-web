//! Type definitions for field-to-field comparisons
//!
//! Represents field-to-field comparison expressions used in JOIN conditions and HAVING clauses.

/// Comparison operators for field-to-field comparisons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
	/// Equal (=)
	Eq,
	/// Not equal (!=)
	Ne,
	/// Greater than (>)
	Gt,
	/// Greater than or equal (>=)
	Gte,
	/// Less than (<)
	Lt,
	/// Less than or equal (<=)
	Lte,
}

/// Field reference (with table alias support)
#[derive(Debug, Clone)]
pub enum FieldRef {
	/// Table field
	Field {
		/// Table alias (e.g., "u1", "u2")
		table_alias: Option<String>,
		/// Field path (e.g., ["user_id"], ["posts", "id"])
		field_path: Vec<String>,
	},
	/// Raw SQL operand emitted verbatim, such as a numeric constant or `NULL`.
	///
	/// # Security
	///
	/// The text is neither quoted, escaped, nor bound as a parameter. Pass only
	/// fixed, trusted SQL; never build it from request data or other
	/// attacker-controlled input.
	Value(String),
}

impl FieldRef {
	/// Create a field reference
	pub fn field(field_path: Vec<String>) -> Self {
		Self::Field {
			table_alias: None,
			field_path,
		}
	}

	/// Create a field reference with table alias
	pub fn field_with_alias(table_alias: String, field_path: Vec<String>) -> Self {
		Self::Field {
			table_alias: Some(table_alias),
			field_path,
		}
	}

	/// Create a raw SQL operand that is emitted verbatim.
	///
	/// # Security
	///
	/// `value` is not quoted, escaped, or bound. Pass only fixed, trusted SQL;
	/// see [`FieldRef::Value`].
	pub fn value(value: String) -> Self {
		Self::Value(value)
	}

	/// Set table alias
	pub fn with_alias(mut self, alias: &str) -> Self {
		if let Self::Field {
			ref mut table_alias,
			..
		} = self
		{
			*table_alias = Some(alias.to_string());
		}
		self
	}
}

/// Field-to-field comparison expression
///
/// # Examples
///
/// ```no_run
/// use reinhardt_db::orm::query_fields::comparison::*;
///
/// // u1.id < u2.id
/// let comparison = FieldComparison::new(
///     FieldRef::field_with_alias("u1".to_string(), vec!["id".to_string()]),
///     FieldRef::field_with_alias("u2".to_string(), vec!["id".to_string()]),
///     ComparisonOperator::Lt,
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FieldComparison {
	/// Left-hand side field reference
	pub left: FieldRef,
	/// Right-hand side field reference
	pub right: FieldRef,
	/// Comparison operator
	pub op: ComparisonOperator,
}

impl FieldComparison {
	/// Create a field comparison expression
	pub fn new(left: FieldRef, right: FieldRef, op: ComparisonOperator) -> Self {
		Self { left, right, op }
	}
}
