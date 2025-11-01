//! Query filtering for proxies

use crate::proxy::ScalarValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOp {
	Eq,
	Ne,
	Lt,
	Le,
	Gt,
	Ge,
	In,
	NotIn,
	Contains,
	StartsWith,
	EndsWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
	pub field: String,
	pub op: FilterOp,
	pub value: String,
}

impl FilterCondition {
	pub fn new(field: String, op: FilterOp, value: String) -> Self {
		Self { field, op, value }
	}

	pub fn matches(&self, scalar: &ScalarValue) -> bool {
		let scalar_str = match scalar {
			ScalarValue::String(s) => s.clone(),
			ScalarValue::Integer(i) => i.to_string(),
			ScalarValue::Float(f) => f.to_string(),
			ScalarValue::Boolean(b) => b.to_string(),
			ScalarValue::Null => "null".to_string(),
		};

		match self.op {
			FilterOp::Eq => scalar_str == self.value,
			FilterOp::Ne => scalar_str != self.value,
			FilterOp::Lt => scalar_str < self.value,
			FilterOp::Le => scalar_str <= self.value,
			FilterOp::Gt => scalar_str > self.value,
			FilterOp::Ge => scalar_str >= self.value,
			FilterOp::In => self.value.contains(&scalar_str),
			FilterOp::NotIn => !self.value.contains(&scalar_str),
			FilterOp::Contains => scalar_str.contains(&self.value),
			FilterOp::StartsWith => scalar_str.starts_with(&self.value),
			FilterOp::EndsWith => scalar_str.ends_with(&self.value),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
	pub conditions: Vec<FilterCondition>,
}

impl QueryFilter {
	pub fn new() -> Self {
		Self {
			conditions: Vec::new(),
		}
	}

	pub fn add_condition(&mut self, condition: FilterCondition) {
		self.conditions.push(condition);
	}
}

impl Default for QueryFilter {
	fn default() -> Self {
		Self::new()
	}
}
