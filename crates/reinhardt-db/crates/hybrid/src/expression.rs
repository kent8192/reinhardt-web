//! SQL expression builders

use serde::{Deserialize, Serialize};

/// Represents a SQL expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlExpression {
    pub sql: String,
}

impl SqlExpression {
    /// Creates a new SQL expression from a string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::expression::SqlExpression;
    ///
    /// let expr = SqlExpression::new("SELECT * FROM users");
    /// assert_eq!(expr.sql, "SELECT * FROM users");
    ///
    /// // Also works with String
    /// let expr2 = SqlExpression::new(String::from("COUNT(*)"));
    /// assert_eq!(expr2.sql, "COUNT(*)");
    /// ```
    pub fn new(sql: impl Into<String>) -> Self {
        Self { sql: sql.into() }
    }
    /// Creates a CONCAT SQL expression from multiple parts
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::expression::SqlExpression;
    ///
    /// let expr = SqlExpression::concat(&["first_name", "' '", "last_name"]);
    /// assert_eq!(expr.sql, "CONCAT(first_name, ' ', last_name)");
    ///
    /// // Single part
    /// let expr2 = SqlExpression::concat(&["column1"]);
    /// assert_eq!(expr2.sql, "CONCAT(column1)");
    /// ```
    pub fn concat(parts: &[&str]) -> Self {
        Self {
            sql: format!("CONCAT({})", parts.join(", ")),
        }
    }
    /// Creates a LOWER SQL expression for case-insensitive operations
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::expression::SqlExpression;
    ///
    /// let expr = SqlExpression::lower("email");
    /// assert_eq!(expr.sql, "LOWER(email)");
    /// ```
    pub fn lower(column: &str) -> Self {
        Self {
            sql: format!("LOWER({})", column),
        }
    }
    /// Creates an UPPER SQL expression for case-insensitive operations
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::expression::SqlExpression;
    ///
    /// let expr = SqlExpression::upper("name");
    /// assert_eq!(expr.sql, "UPPER(name)");
    /// ```
    pub fn upper(column: &str) -> Self {
        Self {
            sql: format!("UPPER({})", column),
        }
    }
    /// Creates a COALESCE SQL expression to handle NULL values
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::expression::SqlExpression;
    ///
    /// let expr = SqlExpression::coalesce("middle_name", "'N/A'");
    /// assert_eq!(expr.sql, "COALESCE(middle_name, 'N/A')");
    /// ```
    pub fn coalesce(column: &str, default: &str) -> Self {
        Self {
            sql: format!("COALESCE({}, {})", column, default),
        }
    }
}

/// Trait for types that can be converted to SQL expressions
pub trait Expression {
    fn to_sql(&self) -> String;
}

impl Expression for SqlExpression {
    fn to_sql(&self) -> String {
        self.sql.clone()
    }
}

impl Expression for String {
    fn to_sql(&self) -> String {
        self.clone()
    }
}

impl Expression for &str {
    fn to_sql(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_expression_lower_unit() {
        let expr = SqlExpression::lower("email");
        assert_eq!(expr.sql, "LOWER(email)");
    }
}
