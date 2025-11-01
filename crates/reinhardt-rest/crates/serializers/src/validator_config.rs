//! Validator configuration for ModelSerializer
//!
//! This module provides configuration structures for managing validators
//! in ModelSerializer instances.

use crate::validators::{UniqueTogetherValidator, UniqueValidator};
use reinhardt_orm::Model;
use std::marker::PhantomData;

/// Configuration for field validators
#[derive(Debug, Clone)]
pub struct ValidatorConfig<M: Model> {
	unique_validators: Vec<UniqueValidator<M>>,
	unique_together_validators: Vec<UniqueTogetherValidator<M>>,
	_phantom: PhantomData<M>,
}

impl<M: Model> ValidatorConfig<M> {
	/// Create a new empty validator configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_serializers::validator_config::ValidatorConfig;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     username: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let config = ValidatorConfig::<User>::new();
	/// ```
	pub fn new() -> Self {
		Self {
			unique_validators: Vec::new(),
			unique_together_validators: Vec::new(),
			_phantom: PhantomData,
		}
	}

	/// Add a unique field validator
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_serializers::validator_config::ValidatorConfig;
	/// use reinhardt_serializers::validators::UniqueValidator;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     username: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let mut config = ValidatorConfig::<User>::new();
	/// config.add_unique_validator(UniqueValidator::new("username"));
	/// ```
	pub fn add_unique_validator(&mut self, validator: UniqueValidator<M>) {
		self.unique_validators.push(validator);
	}

	/// Add a unique together validator
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_serializers::validator_config::ValidatorConfig;
	/// use reinhardt_serializers::validators::UniqueTogetherValidator;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     username: String,
	///     email: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let mut config = ValidatorConfig::<User>::new();
	/// config.add_unique_together_validator(
	///     UniqueTogetherValidator::new(vec!["username", "email"])
	/// );
	/// ```
	pub fn add_unique_together_validator(&mut self, validator: UniqueTogetherValidator<M>) {
		self.unique_together_validators.push(validator);
	}

	/// Get all unique validators
	pub fn unique_validators(&self) -> &[UniqueValidator<M>] {
		&self.unique_validators
	}

	/// Get all unique together validators
	pub fn unique_together_validators(&self) -> &[UniqueTogetherValidator<M>] {
		&self.unique_together_validators
	}

	/// Check if any validators are configured
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_serializers::validator_config::ValidatorConfig;
	/// use reinhardt_serializers::validators::UniqueValidator;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     username: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let mut config = ValidatorConfig::<User>::new();
	/// assert!(!config.has_validators());
	///
	/// config.add_unique_validator(UniqueValidator::new("username"));
	/// assert!(config.has_validators());
	/// ```
	pub fn has_validators(&self) -> bool {
		!self.unique_validators.is_empty() || !self.unique_together_validators.is_empty()
	}
}

impl<M: Model> Default for ValidatorConfig<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::validators::{UniqueTogetherValidator, UniqueValidator};

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestUser {
		id: Option<i64>,
		username: String,
		email: String,
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		fn table_name() -> &'static str {
			"test_users"
		}
		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}
		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_validator_config_new() {
		let config = ValidatorConfig::<TestUser>::new();
		assert_eq!(config.unique_validators().len(), 0);
		assert_eq!(config.unique_together_validators().len(), 0);
		assert!(!config.has_validators());
	}

	#[test]
	fn test_add_unique_validator() {
		let mut config = ValidatorConfig::<TestUser>::new();
		config.add_unique_validator(UniqueValidator::new("username"));

		assert_eq!(config.unique_validators().len(), 1);
		assert!(config.has_validators());
	}

	#[test]
	fn test_add_unique_together_validator() {
		let mut config = ValidatorConfig::<TestUser>::new();
		config
			.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

		assert_eq!(config.unique_together_validators().len(), 1);
		assert!(config.has_validators());
	}

	#[test]
	fn test_multiple_validators() {
		let mut config = ValidatorConfig::<TestUser>::new();
		config.add_unique_validator(UniqueValidator::new("username"));
		config.add_unique_validator(UniqueValidator::new("email"));
		config
			.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

		assert_eq!(config.unique_validators().len(), 2);
		assert_eq!(config.unique_together_validators().len(), 1);
		assert!(config.has_validators());
	}
}
