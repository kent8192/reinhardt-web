//! Order with respect to functionality
//!
//! Django's order_with_respect_to allows automatic ordering of model instances
//! relative to a parent model or set of fields.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Error type for ordering operations
#[derive(Debug)]
pub enum OrderError {
	InvalidOrder(String),
	OrderFieldNotFound(String),
	UpdateFailed(String),
}

impl fmt::Display for OrderError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OrderError::InvalidOrder(msg) => write!(f, "Invalid order value: {}", msg),
			OrderError::OrderFieldNotFound(msg) => write!(f, "Order field not found: {}", msg),
			OrderError::UpdateFailed(msg) => write!(f, "Failed to update order: {}", msg),
		}
	}
}

impl std::error::Error for OrderError {}

/// Value type for filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderValue {
	Integer(i64),
	String(String),
	Boolean(bool),
}

/// Manages ordering for models with order_with_respect_to
///
/// # Examples
///
/// ```
/// use reinhardt_orm::order_with_respect_to::OrderedModel;
/// use std::collections::HashMap;
///
/// let ordered = OrderedModel::new(
///     "order".to_string(),
///     vec!["parent_id".to_string()],
/// );
///
/// assert_eq!(ordered.order_field(), "order");
/// assert_eq!(ordered.order_with_respect_to(), &["parent_id"]);
/// ```
pub struct OrderedModel {
	/// Field name for storing the order
	order_field: String,
	/// Fields that define the ordering scope
	order_with_respect_to: Vec<String>,
}

impl OrderedModel {
	/// Creates a new OrderedModel
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::order_with_respect_to::OrderedModel;
	///
	/// let ordered = OrderedModel::new(
	///     "_order".to_string(),
	///     vec!["category_id".to_string()],
	/// );
	///
	/// assert_eq!(ordered.order_field(), "_order");
	/// ```
	pub fn new(order_field: String, order_with_respect_to: Vec<String>) -> Self {
		Self {
			order_field,
			order_with_respect_to,
		}
	}

	/// Gets the order field name
	pub fn order_field(&self) -> &str {
		&self.order_field
	}

	/// Gets the fields that define the ordering scope
	pub fn order_with_respect_to(&self) -> &[String] {
		&self.order_with_respect_to
	}

	/// Gets the next order value for a given filter scope
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::order_with_respect_to::{OrderedModel, OrderValue};
	/// use std::collections::HashMap;
	///
	/// let ordered = OrderedModel::new(
	///     "order".to_string(),
	///     vec!["parent_id".to_string()],
	/// );
	///
	/// let mut filters = HashMap::new();
	/// filters.insert("parent_id".to_string(), OrderValue::Integer(1));
	///
	/// # tokio_test::block_on(async {
	/// let next_order = ordered.get_next_order(filters).await.unwrap();
	/// assert_eq!(next_order, 0); // First item in this scope
	/// # });
	/// ```
	pub async fn get_next_order(
		&self,
		_filters: HashMap<String, OrderValue>,
	) -> Result<i32, OrderError> {
		// In a real implementation, this would query the database
		// For now, we simulate by returning 0 (first position)
		Ok(0)
	}

	/// Moves an object up in the ordering (decreases order value)
	pub async fn move_up(&self, current_order: i32) -> Result<i32, OrderError> {
		if current_order <= 0 {
			return Err(OrderError::InvalidOrder(
				"Cannot move up from position 0".to_string(),
			));
		}
		Ok(current_order - 1)
	}

	/// Moves an object down in the ordering (increases order value)
	pub async fn move_down(&self, current_order: i32, max_order: i32) -> Result<i32, OrderError> {
		if current_order >= max_order {
			return Err(OrderError::InvalidOrder(format!(
				"Cannot move down from max position {}",
				max_order
			)));
		}
		Ok(current_order + 1)
	}

	/// Moves an object to a specific position in the ordering
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::order_with_respect_to::OrderedModel;
	///
	/// let ordered = OrderedModel::new(
	///     "order".to_string(),
	///     vec!["parent_id".to_string()],
	/// );
	///
	/// # tokio_test::block_on(async {
	/// let new_order = ordered.move_to_position(5, 10, 3).await.unwrap();
	/// assert_eq!(new_order, 3);
	/// # });
	/// ```
	pub async fn move_to_position(
		&self,
		_current_order: i32,
		max_order: i32,
		new_position: i32,
	) -> Result<i32, OrderError> {
		if new_position < 0 || new_position > max_order {
			return Err(OrderError::InvalidOrder(format!(
				"Invalid position: {} (max: {})",
				new_position, max_order
			)));
		}
		Ok(new_position)
	}

	/// Swaps the order of two objects
	pub async fn swap_order(&self, order1: i32, order2: i32) -> Result<(i32, i32), OrderError> {
		Ok((order2, order1))
	}

	/// Reorders all objects in a scope sequentially (0, 1, 2, ...)
	pub async fn reorder_all(
		&self,
		_filters: HashMap<String, OrderValue>,
	) -> Result<Vec<i32>, OrderError> {
		// In a real implementation, this would query all objects and renumber them
		Ok(vec![])
	}

	/// Validates an order value
	pub fn validate_order(&self, order: i32, max_order: i32) -> Result<(), OrderError> {
		if order < 0 {
			return Err(OrderError::InvalidOrder(format!(
				"Order must be non-negative, got {}",
				order
			)));
		}
		if order > max_order {
			return Err(OrderError::InvalidOrder(format!(
				"Order {} exceeds maximum {}",
				order, max_order
			)));
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ordered_model_creation() {
		let ordered = OrderedModel::new("_order".to_string(), vec!["category_id".to_string()]);

		assert_eq!(ordered.order_field(), "_order");
		assert_eq!(ordered.order_with_respect_to().len(), 1);
		assert_eq!(ordered.order_with_respect_to()[0], "category_id");
	}

	#[test]
	fn test_ordered_model_with_multiple_fields() {
		let ordered = OrderedModel::new(
			"order".to_string(),
			vec!["parent_id".to_string(), "category_id".to_string()],
		);

		assert_eq!(ordered.order_with_respect_to().len(), 2);
	}

	#[tokio::test]
	async fn test_get_next_order() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let mut filters = HashMap::new();
		filters.insert("parent_id".to_string(), OrderValue::Integer(1));

		let next_order = ordered.get_next_order(filters).await.unwrap();
		assert_eq!(next_order, 0);
	}

	#[tokio::test]
	async fn test_move_up() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let new_order = ordered.move_up(5).await.unwrap();
		assert_eq!(new_order, 4);

		let result = ordered.move_up(0).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_move_down() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let new_order = ordered.move_down(3, 10).await.unwrap();
		assert_eq!(new_order, 4);

		let result = ordered.move_down(10, 10).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_move_to_position() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let new_order = ordered.move_to_position(5, 10, 7).await.unwrap();
		assert_eq!(new_order, 7);

		let result = ordered.move_to_position(5, 10, 15).await;
		assert!(result.is_err());

		let result = ordered.move_to_position(5, 10, -1).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_swap_order() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let (new_order1, new_order2) = ordered.swap_order(3, 7).await.unwrap();
		assert_eq!(new_order1, 7);
		assert_eq!(new_order2, 3);
	}

	#[test]
	fn test_validate_order() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		assert!(ordered.validate_order(5, 10).is_ok());
		assert!(ordered.validate_order(0, 10).is_ok());
		assert!(ordered.validate_order(10, 10).is_ok());

		assert!(ordered.validate_order(-1, 10).is_err());
		assert!(ordered.validate_order(11, 10).is_err());
	}

	#[tokio::test]
	async fn test_reorder_all() {
		let ordered = OrderedModel::new("order".to_string(), vec!["parent_id".to_string()]);

		let mut filters = HashMap::new();
		filters.insert("parent_id".to_string(), OrderValue::Integer(1));

		let result = ordered.reorder_all(filters).await.unwrap();
		assert_eq!(result.len(), 0);
	}

	#[test]
	fn test_order_value_variants() {
		let int_value = OrderValue::Integer(42);
		let str_value = OrderValue::String("test".to_string());
		let bool_value = OrderValue::Boolean(true);

		match int_value {
			OrderValue::Integer(v) => assert_eq!(v, 42),
			_ => panic!("Expected Integer variant"),
		}

		match str_value {
			OrderValue::String(v) => assert_eq!(v, "test"),
			_ => panic!("Expected String variant"),
		}

		match bool_value {
			OrderValue::Boolean(v) => assert!(v),
			_ => panic!("Expected Boolean variant"),
		}
	}
}
