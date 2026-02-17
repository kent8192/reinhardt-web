//! Existence validators for database foreign keys
//!
//! This validator checks if a referenced value exists in a database table,
//! useful for validating foreign key relationships before insertion/update.

use super::errors::{ValidationError, ValidationResult};
use std::future::Future;
use std::pin::Pin;

/// Type alias for async existence check function
pub type ExistenceCheckFn =
	Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

/// Validator for checking if a foreign key reference exists in the database
///
/// This validator ensures that a referenced value (e.g., user_id, product_id)
/// actually exists in the target table before allowing it to be used as a
/// foreign key.
///
/// # Examples
///
/// ```rust
/// use reinhardt_core::validators::ExistsValidator;
///
/// # async fn example() {
/// // Create a validator that checks if a user_id exists
/// let validator = ExistsValidator::new(
///     "user_id",
///     "users",
///     Box::new(|value| {
///         Box::pin(async move {
///             // Database check logic here
///             // Return true if value exists in the table
///             true
///         })
///     })
/// );
///
/// // Validate that the user exists
/// let result = validator.validate_async("123").await;
/// assert!(result.is_ok());
/// # }
/// ```
pub struct ExistsValidator {
	/// Field name for error messages
	field_name: String,
	/// Table name for error messages
	table_name: String,
	/// Async function to check if value exists in database
	/// Parameter: value -> exists
	check_fn: ExistenceCheckFn,
}

impl ExistsValidator {
	/// Create a new existence validator
	///
	/// # Parameters
	///
	/// * `field_name` - Name of the field being validated (for error messages)
	/// * `table_name` - Name of the referenced table (for error messages)
	/// * `check_fn` - Async function that checks if the value exists in the database.
	///   Returns true if the value exists
	pub fn new(
		field_name: impl Into<String>,
		table_name: impl Into<String>,
		check_fn: ExistenceCheckFn,
	) -> Self {
		Self {
			field_name: field_name.into(),
			table_name: table_name.into(),
			check_fn,
		}
	}

	/// Validate a value asynchronously
	///
	/// # Parameters
	///
	/// * `value` - The value to check for existence in the database
	///
	/// # Returns
	///
	/// * `Ok(())` if the value exists in the referenced table
	/// * `Err(ValidationError::ForeignKeyNotFound)` if the value does not exist
	pub async fn validate_async(&self, value: impl Into<String>) -> ValidationResult<()> {
		let value_str = value.into();
		let exists = (self.check_fn)(value_str.clone()).await;

		if exists {
			Ok(())
		} else {
			Err(ValidationError::ForeignKeyNotFound {
				field: self.field_name.clone(),
				value: value_str,
				table: self.table_name.clone(),
			})
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::collections::HashSet;
	use std::sync::{Arc, Mutex};

	#[rstest]
	#[tokio::test]
	async fn test_exists_validator_value_exists() {
		let existing_ids = Arc::new(Mutex::new(HashSet::from([1, 2, 3])));

		let validator = ExistsValidator::new(
			"user_id",
			"users",
			Box::new(move |value| {
				let existing = existing_ids.clone();
				Box::pin(async move {
					if let Ok(id) = value.parse::<i32>() {
						let ids = existing.lock().unwrap();
						ids.contains(&id)
					} else {
						false
					}
				})
			}),
		);

		// Existing ID should pass
		let result = validator.validate_async("2").await;
		assert!(result.is_ok());

		// Non-existing ID should fail
		let result = validator.validate_async("99").await;
		assert!(result.is_err());
		if let Err(ValidationError::ForeignKeyNotFound {
			field,
			value,
			table,
		}) = result
		{
			assert_eq!(field, "user_id");
			assert_eq!(value, "99");
			assert_eq!(table, "users");
		} else {
			panic!("Expected ForeignKeyNotFound error");
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_exists_validator_empty_database() {
		let validator = ExistsValidator::new(
			"product_id",
			"products",
			Box::new(|_value| Box::pin(async move { false })),
		);

		let result = validator.validate_async("1").await;
		assert!(result.is_err());
		if let Err(ValidationError::ForeignKeyNotFound {
			field,
			value,
			table,
		}) = result
		{
			assert_eq!(field, "product_id");
			assert_eq!(value, "1");
			assert_eq!(table, "products");
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_exists_validator_invalid_value() {
		let validator = ExistsValidator::new(
			"user_id",
			"users",
			Box::new(|value| {
				Box::pin(async move {
					// Only numeric IDs are valid
					value.parse::<i32>().is_ok()
				})
			}),
		);

		// Invalid (non-numeric) ID should fail
		let result = validator.validate_async("abc").await;
		assert!(result.is_err());
	}
}
