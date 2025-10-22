//! Integration tests for validators with serializers
//!
//! Based on Django REST Framework test_validators.py
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
//!     cargo test --test validator_serializer_integration_tests
//! ```

use reinhardt_integration_tests::validator_test_common::*;
use reinhardt_validators::{
    EmailValidator, MaxLengthValidator, MinLengthValidator, UniqueValidator, ValidationError,
    Validator,
};
use std::sync::Arc;

#[cfg(test)]
mod uniqueness_validation_tests {
    use super::*;

    // Based on Django REST Framework: TestUniquenessValidation::test_is_not_unique
    #[tokio::test]
    async fn test_username_uniqueness_validation() {
        let db = TestDatabase::new()
            .await
            .expect("Failed to connect to database");
        db.setup_tables().await.expect("Failed to setup tables");

        // Insert existing user
        db.insert_user("existinguser", "existing@example.com")
            .await
            .expect("Failed to insert user");

        // Test that username validator detects duplicate
        let username_exists = db.username_exists("existinguser").await.unwrap();
        assert!(username_exists, "Username should exist in database");

        // Test with new username
        let new_username_exists = db.username_exists("newuser").await.unwrap();
        assert!(!new_username_exists, "New username should not exist");

        // Cleanup
        db.cleanup().await.expect("Failed to cleanup");
    }

    // Based on Django REST Framework: TestUniquenessValidation::test_is_unique
    #[tokio::test]
    async fn test_email_uniqueness_validation() {
        let db = TestDatabase::new()
            .await
            .expect("Failed to connect to database");
        db.setup_tables().await.expect("Failed to setup tables");

        // Insert existing user
        db.insert_user("user1", "test@example.com")
            .await
            .expect("Failed to insert user");

        // Test that email validator detects duplicate
        let email_exists = db.email_exists("test@example.com").await.unwrap();
        assert!(email_exists, "Email should exist in database");

        // Test with new email
        let new_email_exists = db.email_exists("newemail@example.com").await.unwrap();
        assert!(!new_email_exists, "New email should not exist");

        // Cleanup
        db.cleanup().await.expect("Failed to cleanup");
    }

    // Based on Django REST Framework: TestUniquenessValidation::test_updated_instance_excluded
    #[tokio::test]
    async fn test_updated_instance_excluded_from_uniqueness() {
        let db = TestDatabase::new()
            .await
            .expect("Failed to connect to database");
        db.setup_tables().await.expect("Failed to setup tables");

        // Insert existing user
        let user_id = db
            .insert_user("testuser", "test@example.com")
            .await
            .expect("Failed to insert user");

        // Create a uniqueness validator
        let db_arc = Arc::new(db);
        let db_clone = db_arc.clone();
        let username_validator = UniqueValidator::new(
            "username",
            Box::new(move |value, exclude_id| {
                let db = db_clone.clone();
                Box::pin(async move {
                    db.username_exists_excluding(&value, exclude_id)
                        .await
                        .unwrap_or(false)
                })
            }),
        );

        // When updating the same instance with the same username, validation should succeed
        // (current instance is excluded from uniqueness check)
        let result = username_validator
            .validate_async("testuser", Some(user_id))
            .await;
        assert!(
            result.is_ok(),
            "Updating instance with same value should succeed when instance is excluded"
        );

        // Insert another user
        let other_user_id = db_arc
            .insert_user("otheruser", "other@example.com")
            .await
            .expect("Failed to insert second user");

        // When updating to a username that belongs to another instance, validation should fail
        let result = username_validator
            .validate_async("otheruser", Some(user_id))
            .await;
        assert!(
            result.is_err(),
            "Updating to another instance's value should fail"
        );
        if let Err(ValidationError::NotUnique { field, value }) = result {
            assert_eq!(field, "username");
            assert_eq!(value, "otheruser");
        } else {
            panic!("Expected NotUnique error");
        }

        // When creating a new user with an existing username, validation should fail
        let result = username_validator.validate_async("testuser", None).await;
        assert!(
            result.is_err(),
            "Creating new instance with existing value should fail"
        );

        // When creating a new user with a unique username, validation should succeed
        let result = username_validator.validate_async("newuser", None).await;
        assert!(
            result.is_ok(),
            "Creating new instance with unique value should succeed"
        );

        // Cleanup
        db_arc.cleanup().await.expect("Failed to cleanup");
    }
}

#[cfg(test)]
mod field_validation_tests {
    use super::*;

    // Based on Django forms tests: test_all_errors_get_reported
    #[test]
    fn test_multiple_validators_on_single_field() {
        let min_validator = MinLengthValidator::new(5);
        let max_validator = MaxLengthValidator::new(20);

        // Test value that passes both validators
        let value = "validusername";
        assert!(min_validator.validate(value).is_ok());
        assert!(max_validator.validate(value).is_ok());

        // Test value that fails min length
        let short_value = "usr";
        assert_validation_result(
            min_validator.validate(short_value),
            false,
            Some("too short"),
        );

        // Test value that fails max length
        let long_value = "thisusernameiswaytoolong";
        assert_validation_result(max_validator.validate(long_value), false, Some("too long"));
    }

    // Based on Django test_value_placeholder_with_char_field
    #[test]
    fn test_validator_error_messages_contain_values() {
        let email_validator = EmailValidator::new();

        // Test invalid email
        let result = email_validator.validate("not-an-email");
        assert!(result.is_err());

        if let Err(ValidationError::InvalidEmail(email)) = result {
            assert_eq!(email, "not-an-email");
        } else {
            panic!("Expected InvalidEmail error");
        }
    }

    // Based on Django test_field_validators_can_be_any_iterable
    #[test]
    fn test_combining_multiple_validators() {
        let min_validator = MinLengthValidator::new(3);
        let max_validator = MaxLengthValidator::new(15); // Increased to fit email
        let email_validator = EmailValidator::new();

        // Simulate applying multiple validators to a field
        let test_email = "a@test.co"; // 9 characters: fits in 3-15 range

        // Length validators
        assert!(min_validator.validate(test_email).is_ok());
        assert!(max_validator.validate(test_email).is_ok());

        // Email format validator
        assert!(email_validator.validate(test_email).is_ok());

        // Test with invalid email
        let invalid = "x";
        assert!(min_validator.validate(invalid).is_err());
        assert!(email_validator.validate(invalid).is_err());
    }
}

#[cfg(test)]
mod serializer_field_validation_tests {
    use super::*;

    // Based on DRF: TestValidatorsIntegration
    #[test]
    fn test_email_field_with_validators() {
        let email_validator = EmailValidator::new();
        let min_length = MinLengthValidator::new(5);
        let max_length = MaxLengthValidator::new(255);

        // Valid email that passes all validators
        let valid_email = "user@example.com";
        assert!(email_validator.validate(valid_email).is_ok());
        assert!(min_length.validate(valid_email).is_ok());
        assert!(max_length.validate(valid_email).is_ok());

        // Email shorter than 5 characters - impossible with valid email format
        // Valid emails need: local(1+) + @ + domain(1+) + . + TLD(2+) = minimum 6 characters
        // So we test that short strings fail both validators
        let short_string = "a@b"; // Too short for valid email (no TLD)
        assert!(email_validator.validate(short_string).is_err()); // Invalid format (no TLD)
        assert!(min_length.validate(short_string).is_err()); // Also too short

        // Invalid email format
        let invalid_email = "notanemail";
        assert!(email_validator.validate(invalid_email).is_err());
    }

    #[test]
    fn test_username_field_with_validators() {
        let min_length = MinLengthValidator::new(3);
        let max_length = MaxLengthValidator::new(30);

        // Valid username
        let valid_username = "john_doe_123";
        assert!(min_length.validate(valid_username).is_ok());
        assert!(max_length.validate(valid_username).is_ok());

        // Too short
        let short = "ab";
        assert_validation_result(min_length.validate(short), false, Some("too short"));

        // Too long
        let long = "a".repeat(31);
        assert_validation_result(max_length.validate(&long), false, Some("too long"));
    }
}

#[cfg(test)]
mod validator_composition_tests {
    use super::*;

    // Test that validators can be composed and reused
    #[test]
    fn test_validator_reusability() {
        let email_validator = EmailValidator::new();

        // Use the same validator instance multiple times
        assert!(email_validator.validate("user1@example.com").is_ok());
        assert!(email_validator.validate("user2@example.com").is_ok());
        assert!(email_validator.validate("invalid").is_err());
        assert!(email_validator.validate("user3@example.com").is_ok());
    }

    #[test]
    fn test_validator_independence() {
        let validator1 = MinLengthValidator::new(5);
        let validator2 = MinLengthValidator::new(10);

        let value = "testing";

        // Should pass validator1 but fail validator2
        assert!(validator1.validate(value).is_ok());
        assert!(validator2.validate(value).is_err());
    }
}
