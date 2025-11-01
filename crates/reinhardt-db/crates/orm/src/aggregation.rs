//! Aggregation functions for database queries
//!
//! This module provides Django-inspired aggregation functionality.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Aggregate function types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateFunc {
	/// COUNT aggregation
	Count,
	/// COUNT DISTINCT aggregation
	CountDistinct,
	/// SUM aggregation
	Sum,
	/// AVG aggregation
	Avg,
	/// MAX aggregation
	Max,
	/// MIN aggregation
	Min,
}

impl fmt::Display for AggregateFunc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			AggregateFunc::Count => write!(f, "COUNT"),
			AggregateFunc::CountDistinct => write!(f, "COUNT"),
			AggregateFunc::Sum => write!(f, "SUM"),
			AggregateFunc::Avg => write!(f, "AVG"),
			AggregateFunc::Max => write!(f, "MAX"),
			AggregateFunc::Min => write!(f, "MIN"),
		}
	}
}

/// Aggregate expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregate {
	/// The aggregate function
	pub func: AggregateFunc,
	/// The field to aggregate (None for COUNT(*))
	pub field: Option<String>,
	/// Alias for the result
	pub alias: Option<String>,
	/// Whether this is a DISTINCT aggregation
	pub distinct: bool,
}

impl Aggregate {
	/// Create a COUNT aggregate
	pub fn count(field: Option<&str>) -> Self {
		Self {
			func: AggregateFunc::Count,
			field: field.map(|s| s.to_string()),
			alias: None,
			distinct: false,
		}
	}

	/// Create a COUNT(*) aggregate
	pub fn count_all() -> Self {
		Self {
			func: AggregateFunc::Count,
			field: None,
			alias: None,
			distinct: false,
		}
	}

	/// Create a COUNT DISTINCT aggregate
	pub fn count_distinct(field: &str) -> Self {
		Self {
			func: AggregateFunc::CountDistinct,
			field: Some(field.to_string()),
			alias: None,
			distinct: true,
		}
	}

	/// Create a SUM aggregate
	pub fn sum(field: &str) -> Self {
		Self {
			func: AggregateFunc::Sum,
			field: Some(field.to_string()),
			alias: None,
			distinct: false,
		}
	}

	/// Create an AVG aggregate
	pub fn avg(field: &str) -> Self {
		Self {
			func: AggregateFunc::Avg,
			field: Some(field.to_string()),
			alias: None,
			distinct: false,
		}
	}

	/// Create a MAX aggregate
	pub fn max(field: &str) -> Self {
		Self {
			func: AggregateFunc::Max,
			field: Some(field.to_string()),
			alias: None,
			distinct: false,
		}
	}

	/// Create a MIN aggregate
	pub fn min(field: &str) -> Self {
		Self {
			func: AggregateFunc::Min,
			field: Some(field.to_string()),
			alias: None,
			distinct: false,
		}
	}

	/// Set an alias for this aggregate
	pub fn with_alias(mut self, alias: &str) -> Self {
		self.alias = Some(alias.to_string());
		self
	}

	/// Convert to SQL string
	pub fn to_sql(&self) -> String {
		let mut sql = String::new();

		sql.push_str(&self.func.to_string());
		sql.push('(');

		if self.distinct && self.field.is_some() {
			sql.push_str("DISTINCT ");
		}

		match &self.field {
			Some(field) => sql.push_str(field),
			None => sql.push('*'),
		}

		sql.push(')');

		if let Some(alias) = &self.alias {
			sql.push_str(" AS ");
			sql.push_str(alias);
		}

		sql
	}
}

/// Result of an aggregation
#[derive(Debug, Clone)]
pub enum AggregateValue {
	/// Integer value
	Int(i64),
	/// Float value
	Float(f64),
	/// Null value
	Null,
}

/// Result container for aggregation queries
#[derive(Debug, Clone)]
pub struct AggregateResult {
	/// Map of alias to value
	pub values: std::collections::HashMap<String, AggregateValue>,
}

impl AggregateResult {
	/// Create a new empty result
	pub fn new() -> Self {
		Self {
			values: std::collections::HashMap::new(),
		}
	}

	/// Get a value by alias
	pub fn get(&self, alias: &str) -> Option<&AggregateValue> {
		self.values.get(alias)
	}

	/// Insert a value
	pub fn insert(&mut self, alias: String, value: AggregateValue) {
		self.values.insert(alias, value);
	}
}

impl Default for AggregateResult {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_count_aggregate() {
		let agg = Aggregate::count(Some("id"));
		assert_eq!(agg.to_sql(), "COUNT(id)");
	}

	#[test]
	fn test_count_all_aggregate() {
		let agg = Aggregate::count_all();
		assert_eq!(agg.to_sql(), "COUNT(*)");
	}

	#[test]
	fn test_count_distinct_aggregate() {
		let agg = Aggregate::count_distinct("user_id");
		assert_eq!(agg.to_sql(), "COUNT(DISTINCT user_id)");
	}

	#[test]
	fn test_sum_aggregate() {
		let agg = Aggregate::sum("amount");
		assert_eq!(agg.to_sql(), "SUM(amount)");
	}

	#[test]
	fn test_avg_aggregate() {
		let agg = Aggregate::avg("score");
		assert_eq!(agg.to_sql(), "AVG(score)");
	}

	#[test]
	fn test_max_aggregate() {
		let agg = Aggregate::max("price");
		assert_eq!(agg.to_sql(), "MAX(price)");
	}

	#[test]
	fn test_min_aggregate() {
		let agg = Aggregate::min("age");
		assert_eq!(agg.to_sql(), "MIN(age)");
	}

	#[test]
	fn test_aggregate_with_alias() {
		let agg = Aggregate::sum("amount").with_alias("total_amount");
		assert_eq!(agg.to_sql(), "SUM(amount) AS total_amount");
	}
}
