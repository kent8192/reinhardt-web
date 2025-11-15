//! Integration tests for validators with ORM
//!
//! Based on SQLAlchemy test/orm/test_validators.py
//!
//! **USES TESTCONTAINERS**: These tests use TestContainers for PostgreSQL database.
//! Docker Desktop must be running before executing these tests.

use reinhardt_integration_tests::validator_test_common::*;
use reinhardt_test::fixtures::validator::{
	validator_db_guard, validator_test_db, ValidatorDbGuard,
};
use reinhardt_test::resource::TeardownGuard;
use reinhardt_validators::{
	MaxLengthValidator, MaxValueValidator, MinLengthValidator, MinValueValidator, RangeValidator,
	ValidationError, Validator,
};
use rstest::*;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[cfg(test)]
mod scalar_validation_tests {
	use super::*;

	// Based on SQLAlchemy: ValidatorTest::test_scalar
	#[rstest]
	#[tokio::test]
	async fn test_scalar_field_validation(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<reinhardt_test::fixtures::validator::ValidatorDbGuard>,
	) {
		let (_container, pool, _port, _database_url) = validator_test_db.await;

		// Setup tables using helper function
		setup_test_tables(pool.as_ref()).await;

		// Test username length validation
		let min_validator = MinLengthValidator::new(3);
		let max_validator = MaxLengthValidator::new(100);

		// Valid username
		let valid_username = "john_doe";
		assert!(min_validator.validate(valid_username).is_ok());
		assert!(max_validator.validate(valid_username).is_ok());

		// Insert user with valid username
		let user_id = insert_test_user(pool.as_ref(), valid_username, "john@example.com").await;
		assert!(user_id > 0);

		// Cleanup is automatic via TeardownGuard and container drop
	}

	// Based on SQLAlchemy: test_validators_dict
	#[test]
	fn test_multiple_field_validators() {
		// Simulate validating multiple fields like SQLAlchemy's validator dict
		let username_min = MinLengthValidator::new(3);
		let username_max = MaxLengthValidator::new(30);
		let email_min = MinLengthValidator::new(5);
		let email_max = MaxLengthValidator::new(255);

		// Valid values
		let username = "testuser";
		let email = "test@example.com";

		assert!(username_min.validate(username).is_ok());
		assert!(username_max.validate(username).is_ok());
		assert!(email_min.validate(email).is_ok());
		assert!(email_max.validate(email).is_ok());

		// Invalid values
		let short_username = "ab";
		assert!(username_min.validate(short_username).is_err());

		let short_email = "a@b";
		assert!(email_min.validate(short_email).is_err());
	}
}

#[cfg(test)]
mod numeric_validation_tests {
	use super::*;

	// Test numeric validators with database constraints
	#[test]
	fn test_price_validation() {
		let min_price = MinValueValidator::new(0.0);
		let max_price = MaxValueValidator::new(999999.99);

		// Valid prices
		assert!(min_price.validate(&0.0).is_ok());
		assert!(min_price.validate(&99.99).is_ok());
		assert!(max_price.validate(&500.0).is_ok());

		// Invalid prices
		assert!(min_price.validate(&-10.0).is_err());
		assert!(max_price.validate(&1000000.0).is_err());
	}

	#[test]
	fn test_stock_quantity_validation() {
		let stock_validator = RangeValidator::new(0, 10000);

		// Valid stock quantities
		assert!(stock_validator.validate(&0).is_ok());
		assert!(stock_validator.validate(&100).is_ok());
		assert!(stock_validator.validate(&10000).is_ok());

		// Invalid stock quantities
		assert!(stock_validator.validate(&-1).is_err());
		assert!(stock_validator.validate(&10001).is_err());
	}

	#[test]
	fn test_quantity_validation() {
		// Order quantity must be positive
		let min_quantity = MinValueValidator::new(1);
		let max_quantity = MaxValueValidator::new(100);

		// Valid quantities
		assert!(min_quantity.validate(&1).is_ok());
		assert!(min_quantity.validate(&50).is_ok());
		assert!(max_quantity.validate(&100).is_ok());

		// Invalid quantities
		assert!(min_quantity.validate(&0).is_err());
		assert!(min_quantity.validate(&-5).is_err());
		assert!(max_quantity.validate(&101).is_err());
	}
}

#[cfg(test)]
mod bulk_operation_validation_tests {
	use super::*;

	// Based on SQLAlchemy: test_validator_bulk_collection_set
	#[test]
	fn test_bulk_validation() {
		let username_validator = MinLengthValidator::new(3);

		// Simulate bulk validation
		let usernames = vec!["alice", "bob", "charlie", "david"];

		let results: Vec<_> = usernames
			.iter()
			.map(|name| username_validator.validate(*name))
			.collect();

		// All should pass
		for (i, result) in results.iter().enumerate() {
			assert!(
				result.is_ok(),
				"Username '{}' failed validation",
				usernames[i]
			);
		}

		// Test with invalid usernames
		let invalid_usernames = vec!["ab", "x", ""];
		let invalid_results: Vec<_> = invalid_usernames
			.iter()
			.map(|name| username_validator.validate(*name))
			.collect();

		// All should fail
		for (i, result) in invalid_results.iter().enumerate() {
			assert!(
				result.is_err(),
				"Username '{}' should have failed validation",
				invalid_usernames[i]
			);
		}
	}

	#[test]
	fn test_bulk_email_validation() {
		use reinhardt_validators::EmailValidator;

		let email_validator = EmailValidator::new();

		// Valid emails
		let valid_emails = vec!["user1@example.com", "user2@test.org", "admin@company.co.uk"];

		for email in &valid_emails {
			assert!(
				email_validator.validate(*email).is_ok(),
				"Email '{}' should be valid",
				email
			);
		}

		// Invalid emails
		let invalid_emails = vec![
			"notanemail",
			"@example.com",
			"user@",
			"user space@example.com",
		];

		for email in &invalid_emails {
			assert!(
				email_validator.validate(*email).is_err(),
				"Email '{}' should be invalid",
				email
			);
		}
	}
}

#[cfg(test)]
mod constraint_validation_tests {
	use super::*;

	// Test validators that mirror database constraints
	#[test]
	fn test_not_null_with_min_length() {
		let validator = MinLengthValidator::new(1);

		// Empty string should fail
		assert_validation_result(validator.validate(""), false, Some("too short"));

		// Non-empty should pass
		assert!(validator.validate("a").is_ok());
	}

	#[test]
	fn test_check_constraint_validation() {
		// Simulate CHECK (stock >= 0) constraint
		let stock_validator = MinValueValidator::new(0);

		assert!(stock_validator.validate(&0).is_ok());
		assert!(stock_validator.validate(&100).is_ok());
		assert!(stock_validator.validate(&-1).is_err());

		// Simulate CHECK (quantity > 0) constraint
		let quantity_validator = MinValueValidator::new(1);

		assert!(quantity_validator.validate(&1).is_ok());
		assert!(quantity_validator.validate(&100).is_ok());
		assert!(quantity_validator.validate(&0).is_err());
	}

	#[test]
	fn test_unique_constraint_simulation() {
		// This test verifies in-memory uniqueness checking
		// For actual database UNIQUE constraint validation, see test_unique_constraint_database_violation
		let existing_codes = vec!["PROD001", "PROD002", "PROD003"];

		let new_code = "PROD004";
		assert!(!existing_codes.contains(&new_code));

		let duplicate_code = "PROD001";
		assert!(existing_codes.contains(&duplicate_code));
	}

	#[rstest]
	#[tokio::test]
	async fn test_unique_constraint_database_violation(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<ValidatorDbGuard>,
	) {
		use reinhardt_validators::UniqueValidator;

		let (_container, pool, _port, _database_url) = validator_test_db.await;
		setup_test_tables(pool.as_ref()).await;

		// Insert first user successfully
		let user1_id = insert_test_user(pool.as_ref(), "alice", "alice@example.com").await;
		assert!(user1_id > 0);

		// Create a UniqueValidator for username field
		let pool_clone = pool.clone();
		let username_validator = UniqueValidator::new(
			"username",
			Box::new(move |value, exclude_id| {
				let pool = pool_clone.clone();
				Box::pin(async move {
					let query = if let Some(id) = exclude_id {
						sqlx::query_as::<_, (i64,)>(
							"SELECT COUNT(*) FROM test_users WHERE username = $1 AND id != $2",
						)
						.bind(&value)
						.bind(id)
					} else {
						sqlx::query_as::<_, (i64,)>(
							"SELECT COUNT(*) FROM test_users WHERE username = $1",
						)
						.bind(&value)
					};

					let result = query.fetch_one(pool.as_ref()).await;
					result.map(|(count,)| count > 0).unwrap_or(false)
				})
			}),
		);

		// Validate new username (should pass)
		let result = username_validator.validate_async("bob", None).await;
		assert!(result.is_ok());

		// Validate existing username (should fail with NotUnique error)
		let result = username_validator.validate_async("alice", None).await;
		assert!(result.is_err());
		let error = result.unwrap_err();
		assert_eq!(
			error.to_string(),
			"Field 'username' must be unique. Value 'alice' already exists"
		);

		// Validate updating with same username (should pass when excluding current ID)
		let result = username_validator
			.validate_async("alice", Some(user1_id))
			.await;
		assert!(result.is_ok());

		// Test actual database UNIQUE constraint violation
		// Direct sqlx query to catch UNIQUE constraint error
		let insert_result =
			sqlx::query("INSERT INTO test_users (username, email) VALUES ($1, $2) RETURNING id")
				.bind("alice")
				.bind("different@example.com")
				.fetch_one(pool.as_ref())
				.await;
		assert!(insert_result.is_err());

		// Verify error message contains UNIQUE constraint violation
		let error_message = insert_result.unwrap_err().to_string();
		assert!(
			error_message.contains("unique") || error_message.contains("duplicate"),
			"Expected UNIQUE constraint error, got: {}",
			error_message
		);

		// Cleanup handled automatically by TeardownGuard
	}
}

#[cfg(test)]
mod relationship_validation_tests {
	use super::*;

	// Based on SQLAlchemy: test_validator_backrefs
	#[rstest]
	#[tokio::test]
	async fn test_foreign_key_validation(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<ValidatorDbGuard>,
	) {
		let (_container, pool, _port, _database_url) = validator_test_db.await;
		setup_test_tables(pool.as_ref()).await;

		// Insert user and verify it exists
		let user_id = insert_test_user(pool.as_ref(), "testuser", "test@example.com").await;

		// Simple positivity check (legacy validation)
		assert!(user_id > 0);
		let id_validator = MinValueValidator::new(1);
		assert!(id_validator.validate(&user_id).is_ok());

		// Cleanup handled automatically by TeardownGuard
	}

	#[rstest]
	#[tokio::test]
	async fn test_foreign_key_existence_validation(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<ValidatorDbGuard>,
	) {
		use reinhardt_validators::ExistsValidator;

		let (_container, pool, _port, _database_url) = validator_test_db.await;
		setup_test_tables(pool.as_ref()).await;

		// Insert user and product for valid FK references
		let user_id = insert_test_user(pool.as_ref(), "alice", "alice@example.com").await;
		let product_id = insert_test_product(pool.as_ref(), "Laptop", "PROD001", 999.99, 10).await;

		// Create ExistsValidator for user_id
		let user_pool = pool.clone();
		let user_validator = ExistsValidator::new(
			"user_id",
			"users",
			Box::new(move |value| {
				let pool = user_pool.clone();
				Box::pin(async move {
					if let Ok(id) = value.parse::<i32>() {
						let result = sqlx::query_as::<_, (i64,)>(
							"SELECT COUNT(*) FROM test_users WHERE id = $1",
						)
						.bind(id)
						.fetch_one(pool.as_ref())
						.await;
						result.map(|(count,)| count > 0).unwrap_or(false)
					} else {
						false
					}
				})
			}),
		);

		// Create ExistsValidator for product_id
		let product_pool = pool.clone();
		let product_validator = ExistsValidator::new(
			"product_id",
			"products",
			Box::new(move |value| {
				let pool = product_pool.clone();
				Box::pin(async move {
					if let Ok(id) = value.parse::<i32>() {
						let result = sqlx::query_as::<_, (i64,)>(
							"SELECT COUNT(*) FROM test_products WHERE id = $1",
						)
						.bind(id)
						.fetch_one(pool.as_ref())
						.await;
						result.map(|(count,)| count > 0).unwrap_or(false)
					} else {
						false
					}
				})
			}),
		);

		// Validate existing user_id (should pass)
		let result = user_validator.validate_async(user_id.to_string()).await;
		assert!(result.is_ok());

		// Validate non-existing user_id (should fail)
		let result = user_validator.validate_async("99999").await;
		assert!(result.is_err());
		let error = result.unwrap_err();
		assert_eq!(
			error.to_string(),
			"Foreign key reference not found: user_id with value 99999 does not exist in users"
		);

		// Validate existing product_id (should pass)
		let result = product_validator
			.validate_async(product_id.to_string())
			.await;
		assert!(result.is_ok());

		// Validate non-existing product_id (should fail)
		let result = product_validator.validate_async("88888").await;
		assert!(result.is_err());
		let error = result.unwrap_err();
		assert!(error
			.to_string()
			.contains("Foreign key reference not found"));
		assert!(error.to_string().contains("product_id"));
		assert!(error.to_string().contains("88888"));

		// Test actual database FK constraint violation (insert with invalid FK)
		let insert_result = insert_order_with_seaquery(pool.as_ref(), 99999, product_id, 1).await;
		assert!(insert_result.is_err());

		// Verify error message indicates FK constraint violation
		let error_message = insert_result.unwrap_err().to_string();
		assert!(
			error_message.contains("foreign key") || error_message.contains("violates"),
			"Expected FK constraint error, got: {}",
			error_message
		);

		// Cleanup handled automatically by TeardownGuard
	}

	#[rstest]
	#[tokio::test]
	async fn test_foreign_key_update_violation(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<ValidatorDbGuard>,
	) {
		let (_container, pool, _port, _database_url) = validator_test_db.await;
		setup_test_tables(pool.as_ref()).await;

		// Insert user and product
		let user_id = insert_test_user(pool.as_ref(), "bob", "bob@example.com").await;
		let product_id = insert_test_product(pool.as_ref(), "Mouse", "PROD002", 29.99, 50).await;

		// Insert valid order
		let order_id = insert_test_order(pool.as_ref(), user_id, product_id, 2).await;
		assert!(order_id > 0);

		// Attempt to update order with non-existent user_id (should fail)
		let update_result = update_order_user_with_seaquery(pool.as_ref(), order_id, 99999).await;

		assert!(update_result.is_err());
		let error_message = update_result.unwrap_err().to_string();
		assert!(
			error_message.contains("foreign key") || error_message.contains("violates"),
			"Expected FK constraint error on update, got: {}",
			error_message
		);

		// Cleanup handled automatically by TeardownGuard
	}

	#[rstest]
	#[tokio::test]
	async fn test_foreign_key_cascade_delete(
		#[future] validator_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
		_validator_db_guard: TeardownGuard<ValidatorDbGuard>,
	) {
		let (_container, pool, _port, _database_url) = validator_test_db.await;
		setup_test_tables(pool.as_ref()).await;

		// Insert user and product
		let user_id = insert_test_user(pool.as_ref(), "charlie", "charlie@example.com").await;
		let product_id = insert_test_product(pool.as_ref(), "Keyboard", "PROD003", 79.99, 30).await;

		// Insert order referencing both
		let order_id = insert_test_order(pool.as_ref(), user_id, product_id, 1).await;
		assert!(order_id > 0);

		// Attempt to delete user (should fail because of FK constraint)
		// Note: test_orders table does NOT have ON DELETE CASCADE
		let delete_result = delete_user_with_seaquery(pool.as_ref(), user_id).await;

		assert!(delete_result.is_err());
		let error_message = delete_result.unwrap_err().to_string();
		assert!(
			error_message.contains("foreign key")
				|| error_message.contains("violates")
				|| error_message.contains("still referenced"),
			"Expected FK constraint error on delete, got: {}",
			error_message
		);

		// Verify user still exists (delete was prevented)
		let user_exists = check_user_exists(pool.as_ref(), user_id).await;
		assert!(user_exists);

		// Cleanup handled automatically by TeardownGuard
	}
}

#[cfg(test)]
mod validator_composition_with_orm_tests {
	use super::*;

	#[test]
	fn test_combined_validators_for_model_field() {
		// Simulate validating a model field with multiple validators
		struct UserValidation {
			username_min: MinLengthValidator,
			username_max: MaxLengthValidator,
		}

		impl UserValidation {
			fn new() -> Self {
				Self {
					username_min: MinLengthValidator::new(3),
					username_max: MaxLengthValidator::new(30),
				}
			}

			fn validate_username(&self, username: &str) -> Result<(), Vec<ValidationError>> {
				let mut errors = Vec::new();

				if let Err(e) = self.username_min.validate(username) {
					errors.push(e);
				}
				if let Err(e) = self.username_max.validate(username) {
					errors.push(e);
				}

				if errors.is_empty() {
					Ok(())
				} else {
					Err(errors)
				}
			}
		}

		let validation = UserValidation::new();

		// Valid username
		assert!(validation.validate_username("john_doe").is_ok());

		// Too short
		let result = validation.validate_username("ab");
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().len(), 1);

		// Too long
		let long_username = "a".repeat(50);
		let result = validation.validate_username(&long_username);
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().len(), 1);
	}
}
