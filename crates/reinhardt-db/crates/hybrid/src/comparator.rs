//! Comparator trait for custom comparison logic

/// A comparator that can be used to customize SQL comparison operations
pub trait Comparator {
    /// The type of the expression being compared
    type Expression;

    /// Create a new comparator with the given expression
    fn new(expression: Self::Expression) -> Self;

    /// Get the underlying expression
    fn expression(&self) -> &Self::Expression;

    /// Generate SQL for equality comparison
    fn eq(&self, other: &str) -> String;

    /// Generate SQL for inequality comparison
    fn ne(&self, other: &str) -> String {
        format!("NOT ({})", self.eq(other))
    }

    /// Generate SQL for less than comparison
    fn lt(&self, other: &str) -> String;

    /// Generate SQL for less than or equal comparison
    fn le(&self, other: &str) -> String;

    /// Generate SQL for greater than comparison
    fn gt(&self, other: &str) -> String;

    /// Generate SQL for greater than or equal comparison
    fn ge(&self, other: &str) -> String;
}

/// A comparator that converts both sides to uppercase before comparing
pub struct UpperCaseComparator {
    expression: String,
}

impl Comparator for UpperCaseComparator {
    type Expression = String;

    fn new(expression: String) -> Self {
        Self { expression }
    }

    fn expression(&self) -> &String {
        &self.expression
    }

    fn eq(&self, other: &str) -> String {
        format!("UPPER({}) = UPPER({})", self.expression, other)
    }

    fn lt(&self, other: &str) -> String {
        format!("UPPER({}) < UPPER({})", self.expression, other)
    }

    fn le(&self, other: &str) -> String {
        format!("UPPER({}) <= UPPER({})", self.expression, other)
    }

    fn gt(&self, other: &str) -> String {
        format!("UPPER({}) > UPPER({})", self.expression, other)
    }

    fn ge(&self, other: &str) -> String {
        format!("UPPER({}) >= UPPER({})", self.expression, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppercase_comparator_eq() {
        let comparator = UpperCaseComparator::new("table.column".to_string());
        assert_eq!(
            comparator.eq("'value'"),
            "UPPER(table.column) = UPPER('value')"
        );
    }

    #[test]
    fn test_uppercase_comparator_ne() {
        let comparator = UpperCaseComparator::new("table.column".to_string());
        assert_eq!(
            comparator.ne("'value'"),
            "NOT (UPPER(table.column) = UPPER('value'))"
        );
    }

    #[test]
    fn test_uppercase_comparator_lt() {
        let comparator = UpperCaseComparator::new("table.column".to_string());
        assert_eq!(
            comparator.lt("'value'"),
            "UPPER(table.column) < UPPER('value')"
        );
    }
}
