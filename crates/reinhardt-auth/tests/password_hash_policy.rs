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

#[derive(Clone)]
struct DefaultIdentifierHasher;

impl PasswordHasher for DefaultIdentifierHasher {
	fn hash(&self, password: &str) -> Result<String, Error> {
		Ok(format!("legacy${password}"))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, Error> {
		Ok(hash == format!("legacy${password}"))
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

#[test]
fn policy_rehashes_legacy_hashers_with_default_identifier_methods() {
	let policy =
		PasswordHashPolicy::new(PrefixHasher::new("new")).with_legacy(DefaultIdentifierHasher);

	let result = policy
		.verify_with_update("secret", "legacy$secret")
		.expect("default identifier legacy hasher should be checked");

	assert_eq!(
		result,
		PasswordVerification::ValidNeedsRehash {
			updated_hash: "new$secret".to_string(),
		}
	);
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

#[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
mod bcrypt_policy_tests {
	use reinhardt_auth::{
		Argon2Hasher, BcryptHasher, PasswordHashPolicy, PasswordHasher, PasswordVerification,
	};

	#[test]
	fn policy_rehashes_argon2_to_bcrypt_when_bcrypt_is_preferred() {
		let password = "secret";
		let argon2 = Argon2Hasher::new();
		let bcrypt = BcryptHasher::new();
		let stored_hash = argon2
			.hash(password)
			.expect("Argon2 should hash the password");
		let policy = PasswordHashPolicy::new(bcrypt.clone()).with_legacy(Argon2Hasher::new());

		let result = policy
			.verify_with_update(password, &stored_hash)
			.expect("policy verification should succeed");

		let PasswordVerification::ValidNeedsRehash { updated_hash } = result else {
			panic!("Argon2 legacy hash should be rehashed with bcrypt");
		};
		assert!(bcrypt.identify(&updated_hash));
		assert!(
			bcrypt
				.verify(password, &updated_hash)
				.expect("bcrypt should verify updated hash")
		);
	}

	#[test]
	fn policy_rehashes_bcrypt_to_argon2_when_argon2_is_preferred() {
		let password = "secret";
		let bcrypt = BcryptHasher::new();
		let argon2 = Argon2Hasher::new();
		let stored_hash = bcrypt
			.hash(password)
			.expect("bcrypt should hash the password");
		let policy = PasswordHashPolicy::new(argon2.clone()).with_legacy(BcryptHasher::new());

		let result = policy
			.verify_with_update(password, &stored_hash)
			.expect("policy verification should succeed");

		let PasswordVerification::ValidNeedsRehash { updated_hash } = result else {
			panic!("bcrypt legacy hash should be rehashed with Argon2");
		};
		assert!(argon2.identify(&updated_hash));
		assert!(
			argon2
				.verify(password, &updated_hash)
				.expect("Argon2 should verify updated hash")
		);
	}

	#[test]
	fn bcrypt_hasher_roundtrips_password_at_low_cost() {
		let hasher = BcryptHasher::with_cost(4);
		let hash = hasher
			.hash("secret")
			.expect("bcrypt should hash the password");

		assert!(
			hasher
				.verify("secret", &hash)
				.expect("bcrypt should verify the right password")
		);
		assert!(
			!hasher
				.verify("wrong", &hash)
				.expect("bcrypt should reject the wrong password")
		);
	}

	#[test]
	fn bcrypt_hasher_rejects_passwords_over_72_bytes() {
		let hasher = BcryptHasher::with_cost(4);
		let max_password = "x".repeat(72);
		let overlong_password = "x".repeat(73);
		let hash = hasher
			.hash(&max_password)
			.expect("72-byte bcrypt password should hash");

		assert!(
			hasher.hash(&overlong_password).is_err(),
			"bcrypt should reject overlong passwords before hashing"
		);
		assert!(
			hasher.verify(&overlong_password, &hash).is_err(),
			"bcrypt should reject overlong password candidates before verification"
		);
	}

	#[test]
	#[should_panic(expected = "bcrypt cost must be in")]
	fn bcrypt_hasher_rejects_too_low_cost_on_construction() {
		let _ = BcryptHasher::with_cost(3);
	}

	#[test]
	#[should_panic(expected = "bcrypt cost must be in")]
	fn bcrypt_hasher_rejects_too_high_cost_on_construction() {
		let _ = BcryptHasher::with_cost(32);
	}

	#[test]
	fn bcrypt_hasher_identify_rejects_malformed_prefix() {
		assert!(!BcryptHasher::default().identify("$2b$"));
	}

	#[test]
	fn bcrypt_hasher_identify_rejects_out_of_range_costs() {
		let hash = BcryptHasher::with_cost(4)
			.hash("secret")
			.expect("bcrypt should hash the password");
		let hasher = BcryptHasher::default();

		for invalid_cost in ["03", "32"] {
			let malformed_hash = format!("{}{}{}", &hash[..4], invalid_cost, &hash[6..]);

			assert!(!hasher.identify(&malformed_hash));
			assert!(
				!hasher
					.must_update(&malformed_hash)
					.expect("invalid bcrypt cost should not request an update")
			);
		}
	}

	#[test]
	fn bcrypt_hasher_identifies_supported_valid_prefixes() {
		let hash = BcryptHasher::with_cost(4)
			.hash("secret")
			.expect("bcrypt should hash the password");
		let parts = hash
			.parse::<bcrypt::HashParts>()
			.expect("bcrypt hash should parse into parts");
		let hasher = BcryptHasher::default();

		for version in [
			bcrypt::Version::TwoA,
			bcrypt::Version::TwoB,
			bcrypt::Version::TwoX,
			bcrypt::Version::TwoY,
		] {
			let formatted_hash = parts.format_for_version(version);

			assert!(hasher.identify(&formatted_hash));
		}
	}

	#[test]
	fn bcrypt_hasher_requests_rehash_for_non_two_b_prefixes() {
		let hasher = BcryptHasher::with_cost(4);
		let hash = hasher
			.hash("secret")
			.expect("bcrypt should hash the password");
		let parts = hash
			.parse::<bcrypt::HashParts>()
			.expect("bcrypt hash should parse into parts");

		let two_b_hash = parts.format_for_version(bcrypt::Version::TwoB);
		assert!(
			!hasher
				.must_update(&two_b_hash)
				.expect("same-cost 2b hash should be current")
		);

		for version in [
			bcrypt::Version::TwoA,
			bcrypt::Version::TwoX,
			bcrypt::Version::TwoY,
		] {
			let formatted_hash = parts.format_for_version(version);

			assert!(
				hasher
					.must_update(&formatted_hash)
					.expect("same-cost non-2b hash should be parsed")
			);
		}
	}

	#[test]
	fn bcrypt_hasher_detects_cost_drift() {
		let hash = BcryptHasher::with_cost(4)
			.hash("secret")
			.expect("bcrypt should hash the password");

		assert!(
			BcryptHasher::with_cost(5)
				.must_update(&hash)
				.expect("bcrypt hash should parse for policy comparison")
		);
	}

	#[test]
	fn bcrypt_hasher_preserves_higher_cost_hashes() {
		let hash = BcryptHasher::with_cost(5)
			.hash("secret")
			.expect("higher-cost bcrypt should hash the password");
		let parts = hash
			.parse::<bcrypt::HashParts>()
			.expect("bcrypt hash should parse into parts");
		let legacy_prefix_hash = parts.format_for_version(bcrypt::Version::TwoA);

		assert!(
			!BcryptHasher::with_cost(4)
				.must_update(&hash)
				.expect("higher-cost bcrypt hash should parse")
		);
		assert!(
			!BcryptHasher::with_cost(4)
				.must_update(&legacy_prefix_hash)
				.expect("higher-cost legacy-prefix bcrypt hash should parse")
		);
	}
}
