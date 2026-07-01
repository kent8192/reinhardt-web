//! Password hash policy upgrade tests.

use reinhardt_auth::{PasswordHashPolicy, PasswordHasher, PasswordVerification};
use reinhardt_core::exception::Error;

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
