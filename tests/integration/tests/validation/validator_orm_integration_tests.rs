//! Integration tests for validators with ORM
//!
//! Based on SQLAlchemy test/orm/test_validators.py
//!
//! **REQUIRES DATABASE**: These tests require a running PostgreSQL database.
//!
//! ## Manual Setup
//!
//! ```bash
//! # Start PostgreSQL container
//! docker run --rm -d -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres:17-alpine
//!
//! # Run tests
//! TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres \
//!     cargo test --test validator_orm_integration_tests
//! ```

use reinhardt_integration_tests::validator_test_common::*;
use reinhardt_validators::{
	MaxLengthValidator, MaxValueValidator, MinLengthValidator, MinValueValidator, RangeValidator,
	ValidationError, Validator,
};

#[cfg(test)]
mod scalar_validation_tests {
	use super::*;

	// Based on SQLAlchemy: ValidatorTest::test_scalar
	#[tokio::test]
	async fn test_scalar_field_validation() {
		let db = TestDatabase::new()
			.await
			.expect("Failed to connect to database");
		db.setup_tables().await.expect("Failed to setup tables");

		// Test username length validation
		let min_validator = MinLengthValidator::new(3);
		let max_validator = MaxLengthValidator::new(100);

		// Valid username
		let valid_username = "john_doe";
		assert!(min_validator.validate(valid_username).is_ok());
		assert!(max_validator.validate(valid_username).is_ok());

		// Insert user with valid username
		let user_id = db
			.insert_user(valid_username, "john@example.com")
			.await
			.expect("Failed to insert user");
		assert!(user_id > 0);

		// Cleanup
		db.cleanup().await.expect("Failed to cleanup");
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
		// In a real implementation, this would check database uniqueness
		// Here we simulate with a simple in-memory check
		let existing_codes = vec!["PROD001", "PROD002", "PROD003"];

		let new_code = "PROD004";
		assert!(!existing_codes.contains(&new_code));

		let duplicate_code = "PROD001";
		assert!(existing_codes.contains(&duplicate_code));
	}
}

#[cfg(test)]
mod relationship_validation_tests {
	use super::*;

	// Based on SQLAlchemy: test_validator_backrefs
	#[tokio::test]
	async fn test_foreign_key_validation() {
		let db = TestDatabase::new()
			.await
			.expect("Failed to connect to database");
		db.setup_tables().await.expect("Failed to setup tables");

		// Insert user and verify it exists
		let user_id = db
			.insert_user("testuser", "test@example.com")
			.await
			.expect("Failed to insert user");

		// In a real implementation, we would validate that user_id exists
		// before creating an order with this user_id
		assert!(user_id > 0);

		// Simulate validation: ensure user_id is positive
		let id_validator = MinValueValidator::new(1);
		assert!(id_validator.validate(&user_id).is_ok());

		// Cleanup
		db.cleanup().await.expect("Failed to cleanup");
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
