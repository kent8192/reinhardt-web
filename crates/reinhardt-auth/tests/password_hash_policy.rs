//! Password hash policy upgrade tests.

use chrono::{DateTime, Utc};
use reinhardt_auth::{
	BaseUser, PasswordCheck, PasswordHashPolicy, PasswordHasher, PasswordVerification,
};
use reinhardt_core::exception::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct PrefixHasher {
	algorithm: &'static str,
	stale: bool,
	accepted_password: Option<&'static str>,
}

impl PrefixHasher {
	fn new(algorithm: &'static str) -> Self {
		Self {
			algorithm,
			stale: false,
			accepted_password: None,
		}
	}

	fn stale(algorithm: &'static str) -> Self {
		Self {
			algorithm,
			stale: true,
			accepted_password: None,
		}
	}

	fn with_accepted_password(mut self, accepted_password: &'static str) -> Self {
		self.accepted_password = Some(accepted_password);
		self
	}
}

impl PasswordHasher for PrefixHasher {
	fn hash(&self, password: &str) -> Result<String, Error> {
		Ok(format!("{}${}", self.algorithm, password))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, Error> {
		let accepted_password = self.accepted_password.unwrap_or(password);

		Ok(hash == format!("{}${}", self.algorithm, accepted_password))
	}

	fn algorithm(&self) -> Option<&'static str> {
		Some(self.algorithm)
	}

	fn identify(&self, hash: &str) -> bool {
		hash.strip_prefix(self.algorithm)
			.is_some_and(|remaining| remaining.starts_with('$'))
	}

	fn must_update(&self, hash: &str) -> Result<bool, Error> {
		Ok(self.identify(hash) && self.stale)
	}
}

impl Default for PrefixHasher {
	fn default() -> Self {
		Self::new("new")
	}
}

#[test]
fn policy_accepts_current_preferred_hash_without_update() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));

	let result = policy
		.verify_with_update("secret", "new$secret")
		.expect("policy verification should succeed");

	assert_eq!(result, PasswordVerification::Valid);
}

#[test]
fn policy_rehashes_legacy_algorithm_with_preferred_hasher() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));

	let result = policy
		.verify_with_update("secret", "old$secret")
		.expect("policy verification should succeed");

	assert_eq!(
		result,
		PasswordVerification::ValidNeedsRehash {
			updated_hash: "new$secret".to_string(),
		}
	);
}

#[test]
fn policy_rehashes_stale_preferred_parameters() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::stale("new")).with_legacy(PrefixHasher::new("old"));

	let result = policy
		.verify_with_update("secret", "new$secret")
		.expect("policy verification should succeed");

	assert_eq!(
		result,
		PasswordVerification::ValidNeedsRehash {
			updated_hash: "new$secret".to_string(),
		}
	);
}

#[test]
fn policy_does_not_update_wrong_passwords() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));

	let result = policy
		.verify_with_update("wrong", "old$secret")
		.expect("policy verification should succeed");

	assert_eq!(result, PasswordVerification::Invalid);
}

#[test]
fn policy_uses_first_matching_legacy_hasher() {
	let policy = PasswordHashPolicy::new(PrefixHasher::new("new"))
		.with_legacy(PrefixHasher::new("old").with_accepted_password("different"))
		.with_legacy(PrefixHasher::new("old"));

	let result = policy
		.verify_with_update("secret", "old$secret")
		.expect("policy verification should succeed");

	assert_eq!(result, PasswordVerification::Invalid);
}

#[test]
fn policy_rejects_unknown_algorithm_without_rehashing() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));

	let result = policy.verify_with_update("secret", "unknown$secret");

	assert!(result.is_err(), "unknown algorithms should not be accepted");
}

#[derive(Serialize, Deserialize)]
struct PolicyUser {
	username: String,
	password_hash: Option<String>,
	last_login: Option<DateTime<Utc>>,
	is_active: bool,
}

impl BaseUser for PolicyUser {
	type PrimaryKey = String;
	type Hasher = PrefixHasher;

	fn get_username_field() -> &'static str {
		"username"
	}

	fn get_username(&self) -> &str {
		&self.username
	}

	fn password_hash(&self) -> Option<&str> {
		self.password_hash.as_deref()
	}

	fn set_password_hash(&mut self, hash: String) {
		self.password_hash = Some(hash);
	}

	fn last_login(&self) -> Option<DateTime<Utc>> {
		self.last_login
	}

	fn set_last_login(&mut self, time: DateTime<Utc>) {
		self.last_login = Some(time);
	}

	fn is_active(&self) -> bool {
		self.is_active
	}
}

fn policy_user_with_hash(hash: &str) -> PolicyUser {
	PolicyUser {
		username: "alice".to_string(),
		password_hash: Some(hash.to_string()),
		last_login: None,
		is_active: true,
	}
}

#[test]
fn base_user_updates_legacy_hash_in_memory() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));
	let mut user = policy_user_with_hash("old$secret");

	let result = user
		.check_password_with_policy_update("secret", &policy)
		.expect("policy verification should succeed");

	assert_eq!(result, PasswordCheck::ValidUpdated);
	assert_eq!(user.password_hash(), Some("new$secret"));
}

#[test]
fn base_user_does_not_update_wrong_password() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(PrefixHasher::new("old"));
	let mut user = policy_user_with_hash("old$secret");

	let result = user
		.check_password_with_policy_update("wrong", &policy)
		.expect("policy verification should succeed");

	assert_eq!(result, PasswordCheck::Invalid);
	assert_eq!(user.password_hash(), Some("old$secret"));
}

#[test]
fn base_user_default_update_helper_preserves_single_hasher_compatibility() {
	let mut user = policy_user_with_hash("new$secret");

	let result = user
		.check_password_with_update("secret")
		.expect("password verification should succeed");

	assert_eq!(result, PasswordCheck::Valid);
	assert_eq!(user.password_hash(), Some("new$secret"));
}
