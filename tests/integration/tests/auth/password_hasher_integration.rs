//! Password Hasher Integration Tests
//!
//! Comprehensive tests for the password hashing functionality including:
//! - Boundary value analysis for password lengths
//! - Equivalence partitioning for character sets
//! - Edge cases and error handling
//! - Security properties validation

use reinhardt_auth::{Argon2Hasher, PasswordHasher};
use rstest::*;

// =============================================================================
// Test Fixtures
// =============================================================================

/// Argon2 hasher fixture for all password tests
#[fixture]
fn hasher() -> Argon2Hasher {
	Argon2Hasher::default()
}

// =============================================================================
// Happy Path Tests
// =============================================================================

#[rstest]
fn test_password_hash_and_verify_basic(hasher: Argon2Hasher) {
	// Arrange
	let password = "SecurePassword123!";

	// Act
	let hash = hasher.hash(password).expect("Hashing should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password verification should return true for correct password"
	);
	assert_ne!(hash, password, "Hash should differ from plaintext");
	assert!(
		hash.len() > 50,
		"Hash should be sufficiently long (got {} chars)",
		hash.len()
	);
}

#[rstest]
fn test_same_password_produces_different_hashes(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword123";

	// Act
	let hash1 = hasher.hash(password).expect("First hash should succeed");
	let hash2 = hasher.hash(password).expect("Second hash should succeed");

	// Assert - Due to random salt, hashes should differ
	assert_ne!(
		hash1, hash2,
		"Same password should produce different hashes due to random salt"
	);

	// Both hashes should still verify correctly
	assert!(
		hasher.verify(password, &hash1).unwrap(),
		"First hash should verify"
	);
	assert!(
		hasher.verify(password, &hash2).unwrap(),
		"Second hash should verify"
	);
}

#[rstest]
fn test_wrong_password_fails_verification(hasher: Argon2Hasher) {
	// Arrange
	let correct_password = "CorrectPassword123";
	let wrong_password = "WrongPassword456";

	// Act
	let hash = hasher
		.hash(correct_password)
		.expect("Hashing should succeed");
	let result = hasher
		.verify(wrong_password, &hash)
		.expect("Verification should not error");

	// Assert
	assert!(
		!result,
		"Verification should return false for incorrect password"
	);
}

// =============================================================================
// Boundary Value Analysis Tests
// =============================================================================

#[rstest]
#[case("", "empty password")]
#[case("a", "single character")]
#[case("ab", "two characters")]
fn test_password_hash_minimum_lengths(
	hasher: Argon2Hasher,
	#[case] password: &str,
	#[case] description: &str,
) {
	// Act
	let hash_result = hasher.hash(password);

	// Assert - Even short passwords should hash successfully
	assert!(
		hash_result.is_ok(),
		"Hashing {} should succeed, got error: {:?}",
		description,
		hash_result.err()
	);

	let hash = hash_result.unwrap();
	let verify_result = hasher.verify(password, &hash);
	assert!(
		verify_result.is_ok(),
		"Verification for {} should succeed",
		description
	);
	assert!(
		verify_result.unwrap(),
		"Verification for {} should return true",
		description
	);
}

#[rstest]
#[case(72, "at bcrypt compatibility limit")]
#[case(73, "one beyond bcrypt limit")]
#[case(128, "common max password length")]
#[case(256, "extended password length")]
fn test_password_hash_longer_lengths(
	hasher: Argon2Hasher,
	#[case] length: usize,
	#[case] description: &str,
) {
	// Arrange
	let password: String = "a".repeat(length);

	// Act
	let hash_result = hasher.hash(&password);

	// Assert
	assert!(
		hash_result.is_ok(),
		"Hashing password {} ({} chars) should succeed",
		description,
		length
	);

	let hash = hash_result.unwrap();
	assert!(
		hasher.verify(&password, &hash).unwrap(),
		"Verification for {} should succeed",
		description
	);
}

#[rstest]
fn test_password_hash_very_long_password(hasher: Argon2Hasher) {
	// Arrange - 1000 character password
	let password: String = "a".repeat(1000);

	// Act
	let hash = hasher
		.hash(&password)
		.expect("Hashing very long password should succeed");
	let is_valid = hasher
		.verify(&password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Very long password should verify correctly");
}

#[rstest]
fn test_password_hash_extremely_long_password(hasher: Argon2Hasher) {
	// Arrange - 10000 character password
	let password: String = "x".repeat(10000);

	// Act
	let hash = hasher
		.hash(&password)
		.expect("Hashing extremely long password should succeed");
	let is_valid = hasher
		.verify(&password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Extremely long password should verify correctly");
}

// =============================================================================
// Equivalence Partitioning Tests - Character Sets
// =============================================================================

#[rstest]
#[case("password123", "ASCII alphanumeric")]
#[case("PASSWORD123", "ASCII uppercase")]
#[case("12345678", "numeric only")]
#[case("abcdefgh", "lowercase only")]
fn test_password_hash_ascii_character_sets(
	hasher: Argon2Hasher,
	#[case] password: &str,
	#[case] description: &str,
) {
	// Act
	let hash = hasher
		.hash(password)
		.expect(&format!("Hashing {} should succeed", description));
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password with {} should hash and verify correctly",
		description
	);
}

#[rstest]
#[case("パスワード", "Japanese Hiragana/Katakana")]
#[case("密码测试", "Chinese characters")]
#[case("пароль", "Russian Cyrillic")]
#[case("كلمة السر", "Arabic script")]
#[case("סיסמה", "Hebrew script")]
fn test_password_hash_unicode_scripts(
	hasher: Argon2Hasher,
	#[case] password: &str,
	#[case] description: &str,
) {
	// Act
	let hash = hasher
		.hash(password)
		.expect(&format!("Hashing {} should succeed", description));
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password with {} should hash and verify correctly",
		description
	);
}

#[rstest]
fn test_password_hash_emoji_characters(hasher: Argon2Hasher) {
	// Arrange
	let password = "🔐🔑💪🛡️";

	// Act
	let hash = hasher
		.hash(password)
		.expect("Hashing emoji password should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Emoji password should hash and verify correctly");
}

#[rstest]
#[case("!@#$%^&*()", "common special characters")]
#[case("<>{}[]|\\", "brackets and pipes")]
#[case("`~;:'\",./", "punctuation marks")]
fn test_password_hash_special_characters(
	hasher: Argon2Hasher,
	#[case] password: &str,
	#[case] description: &str,
) {
	// Act
	let hash = hasher
		.hash(password)
		.expect(&format!("Hashing {} should succeed", description));
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password with {} should hash and verify correctly",
		description
	);
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[rstest]
fn test_password_hash_whitespace_only(hasher: Argon2Hasher) {
	// Arrange
	let password = "   \t\n\r   ";

	// Act
	let hash = hasher
		.hash(password)
		.expect("Hashing whitespace-only password should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Whitespace-only password should verify correctly");
}

#[rstest]
fn test_password_hash_internal_whitespace(hasher: Argon2Hasher) {
	// Arrange
	let password = "pass word with spaces";

	// Act
	let hash = hasher.hash(password).expect("Hashing should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Password with internal whitespace should verify");

	// Verify that trimmed version does NOT verify
	let trimmed_verify = hasher.verify("passwordwithspaces", &hash).unwrap();
	assert!(
		!trimmed_verify,
		"Trimmed password should NOT verify against original"
	);
}

#[rstest]
fn test_password_hash_leading_trailing_whitespace(hasher: Argon2Hasher) {
	// Arrange
	let password_with_spaces = "  password  ";
	let password_trimmed = "password";

	// Act
	let hash = hasher
		.hash(password_with_spaces)
		.expect("Hashing should succeed");

	// Assert - These should be treated as different passwords
	assert!(
		hasher.verify(password_with_spaces, &hash).unwrap(),
		"Password with spaces should verify"
	);
	assert!(
		!hasher.verify(password_trimmed, &hash).unwrap(),
		"Trimmed password should NOT verify"
	);
}

#[rstest]
fn test_password_hash_null_byte(hasher: Argon2Hasher) {
	// Arrange - Password containing null byte
	let password = "pass\0word";

	// Act
	let hash = hasher
		.hash(password)
		.expect("Hashing password with null byte should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(is_valid, "Password with null byte should verify correctly");

	// Verify truncated version does NOT verify
	let truncated_verify = hasher.verify("pass", &hash).unwrap();
	assert!(!truncated_verify, "Truncated at null should NOT verify");
}

#[rstest]
fn test_password_hash_control_characters(hasher: Argon2Hasher) {
	// Arrange - Password with various control characters
	let password = "\x01\x02\x03\x04\x05";

	// Act
	let hash = hasher
		.hash(password)
		.expect("Hashing control characters should succeed");
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password with control characters should verify correctly"
	);
}

// =============================================================================
// Error Path Tests
// =============================================================================

#[rstest]
fn test_verify_with_corrupted_hash(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword123";
	let corrupted_hash = "not_a_valid_argon2_hash";

	// Act
	let result = hasher.verify(password, corrupted_hash);

	// Assert
	assert!(
		result.is_err(),
		"Verification with corrupted hash should return error"
	);
}

#[rstest]
fn test_verify_with_empty_hash(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword123";
	let empty_hash = "";

	// Act
	let result = hasher.verify(password, empty_hash);

	// Assert
	assert!(
		result.is_err(),
		"Verification with empty hash should return error"
	);
}

#[rstest]
fn test_verify_with_truncated_hash(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword123";
	let hash = hasher.hash(password).expect("Hashing should succeed");
	let truncated_hash = &hash[..hash.len() / 2];

	// Act
	let result = hasher.verify(password, truncated_hash);

	// Assert
	// Truncated hash should either return error (parse failure) or false (verification failure)
	match result {
		Ok(is_valid) => assert!(!is_valid, "Truncated hash should not verify successfully"),
		Err(_) => {
			// Error is also acceptable for malformed hash (PasswordHash::new() failure)
		}
	}
}

#[rstest]
fn test_verify_with_modified_hash(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword123";
	let hash = hasher.hash(password).expect("Hashing should succeed");

	// Modify a character in the middle of the hash
	let mut modified_hash = hash.clone();
	let mid = modified_hash.len() / 2;
	unsafe {
		let bytes = modified_hash.as_bytes_mut();
		bytes[mid] = if bytes[mid] == b'a' { b'b' } else { b'a' };
	}

	// Act
	let result = hasher.verify(password, &modified_hash);

	// Assert - Either error or false is acceptable
	match result {
		Ok(is_valid) => assert!(!is_valid, "Modified hash should not verify"),
		Err(_) => {} // Error is also acceptable for invalid hash format
	}
}

// =============================================================================
// Decision Table Tests - Password Complexity
// =============================================================================

#[rstest]
#[case(true, true, true, true, "P@ssw0rd!", "all complexity requirements")]
#[case(true, true, true, false, "Password1", "no special chars")]
#[case(true, true, false, true, "Password!", "no numbers")]
#[case(true, false, true, true, "p@ssw0rd!", "no uppercase")]
#[case(false, true, true, true, "P@SSW0RD!", "no lowercase")]
fn test_password_hash_complexity_combinations(
	hasher: Argon2Hasher,
	#[case] has_lowercase: bool,
	#[case] has_uppercase: bool,
	#[case] has_digit: bool,
	#[case] has_special: bool,
	#[case] password: &str,
	#[case] description: &str,
) {
	// Verify password matches expected complexity (sanity check)
	assert_eq!(
		password.chars().any(|c| c.is_lowercase()),
		has_lowercase,
		"Password '{}' lowercase mismatch",
		password
	);
	assert_eq!(
		password.chars().any(|c| c.is_uppercase()),
		has_uppercase,
		"Password '{}' uppercase mismatch",
		password
	);
	assert_eq!(
		password.chars().any(|c| c.is_ascii_digit()),
		has_digit,
		"Password '{}' digit mismatch",
		password
	);
	assert_eq!(
		password.chars().any(|c| !c.is_alphanumeric()),
		has_special,
		"Password '{}' special char mismatch",
		password
	);

	// Act - Hash and verify
	let hash = hasher.hash(password).expect(&format!(
		"Hashing password with {} should succeed",
		description
	));
	let is_valid = hasher
		.verify(password, &hash)
		.expect("Verification should succeed");

	// Assert
	assert!(
		is_valid,
		"Password with {} should hash and verify correctly",
		description
	);
}

// =============================================================================
// State Transition Tests
// =============================================================================

#[rstest]
fn test_password_change_workflow(hasher: Argon2Hasher) {
	// Arrange - Initial password
	let old_password = "OldPassword123";
	let new_password = "NewPassword456";

	// Act - Hash old password
	let old_hash = hasher
		.hash(old_password)
		.expect("Hashing old password should succeed");

	// Verify old password works
	assert!(
		hasher.verify(old_password, &old_hash).unwrap(),
		"Old password should verify with old hash"
	);

	// Hash new password (simulating password change)
	let new_hash = hasher
		.hash(new_password)
		.expect("Hashing new password should succeed");

	// Assert - State after password change
	assert!(
		hasher.verify(new_password, &new_hash).unwrap(),
		"New password should verify with new hash"
	);
	assert!(
		!hasher.verify(old_password, &new_hash).unwrap(),
		"Old password should NOT verify with new hash"
	);
	assert!(
		!hasher.verify(new_password, &old_hash).unwrap(),
		"New password should NOT verify with old hash"
	);
}

#[rstest]
fn test_multiple_hash_operations_independent(hasher: Argon2Hasher) {
	// Arrange - Multiple passwords
	let passwords = [
		"Password1",
		"Password2",
		"Password3",
		"Password4",
		"Password5",
	];

	// Act - Hash all passwords
	let hashes: Vec<String> = passwords
		.iter()
		.map(|p| hasher.hash(p).expect("Hashing should succeed"))
		.collect();

	// Assert - Each password only verifies with its own hash
	for (i, password) in passwords.iter().enumerate() {
		for (j, hash) in hashes.iter().enumerate() {
			let should_match = i == j;
			let result = hasher.verify(password, hash).unwrap();
			assert_eq!(
				result,
				should_match,
				"Password '{}' with hash {} should {} match",
				password,
				j,
				if should_match { "" } else { "NOT " }
			);
		}
	}
}

// =============================================================================
// Use Case Tests
// =============================================================================

#[rstest]
fn test_user_registration_and_login_workflow(hasher: Argon2Hasher) {
	// Arrange - User registers with password
	let registration_password = "UserRegistrationPass123!";

	// Act - Registration: hash the password
	let stored_hash = hasher
		.hash(registration_password)
		.expect("Password hashing during registration should succeed");

	// Login attempt with correct password
	let login_password = "UserRegistrationPass123!";
	let login_result = hasher
		.verify(login_password, &stored_hash)
		.expect("Login verification should not error");

	// Assert
	assert!(login_result, "Login with correct password should succeed");

	// Login attempt with incorrect password
	let wrong_login = hasher.verify("WrongPassword", &stored_hash).unwrap();
	assert!(!wrong_login, "Login with wrong password should fail");
}

#[rstest]
fn test_case_sensitive_passwords(hasher: Argon2Hasher) {
	// Arrange
	let password_lower = "password";
	let password_upper = "PASSWORD";
	let password_mixed = "PaSsWoRd";

	// Act
	let hash = hasher.hash(password_lower).expect("Hashing should succeed");

	// Assert - Passwords are case-sensitive
	assert!(
		hasher.verify(password_lower, &hash).unwrap(),
		"Lowercase should match"
	);
	assert!(
		!hasher.verify(password_upper, &hash).unwrap(),
		"Uppercase should NOT match"
	);
	assert!(
		!hasher.verify(password_mixed, &hash).unwrap(),
		"Mixed case should NOT match"
	);
}

// =============================================================================
// Sanity Tests
// =============================================================================

#[rstest]
fn test_hasher_default_initialization() {
	// Act
	let hasher = Argon2Hasher::default();

	// Assert - Basic functionality works
	let hash = hasher.hash("test").expect("Default hasher should work");
	assert!(!hash.is_empty(), "Hash should not be empty");
	assert!(
		hash.starts_with("$argon2"),
		"Hash should be in Argon2 format"
	);
}

#[rstest]
fn test_hash_format_is_argon2(hasher: Argon2Hasher) {
	// Arrange
	let password = "TestPassword";

	// Act
	let hash = hasher.hash(password).expect("Hashing should succeed");

	// Assert - Verify hash format
	// PHC format: $argon2id$v=19$m=65536,t=3,p=4$salt$hash
	assert!(
		hash.starts_with("$argon2"),
		"Hash should start with $argon2, got: {}",
		&hash[..20.min(hash.len())]
	);
	assert!(
		hash.contains("$v="),
		"Hash should contain version parameter"
	);
	// Parameters are comma-separated in PHC format, e.g., m=65536,t=3,p=4
	assert!(hash.contains("m="), "Hash should contain memory parameter");
	assert!(hash.contains("t="), "Hash should contain time parameter");
	assert!(
		hash.contains("p="),
		"Hash should contain parallelism parameter"
	);
}
