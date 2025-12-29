//! Session CSRF Integration Tests
//!
//! Comprehensive tests for CSRF token management with session storage.
//! These tests verify the integration between CSRF protection and session backends.
//!
//! # Test Categories
//!
//! - Happy path: Token generation, validation, and rotation
//! - Error path: Invalid tokens, missing tokens
//! - State transition: Token lifecycle management
//! - Edge cases: Special characters, concurrent access

use reinhardt_sessions::Session;
use reinhardt_sessions::backends::InMemorySessionBackend;
use reinhardt_sessions::csrf::CsrfSessionManager;
use rstest::*;

// =============================================================================
// Test Fixtures
// =============================================================================

/// Creates a new in-memory session for testing
#[fixture]
fn session() -> Session<InMemorySessionBackend> {
	let backend = InMemorySessionBackend::new();
	Session::new(backend)
}

/// Creates a CSRF session manager with default settings
#[fixture]
fn csrf_manager() -> CsrfSessionManager {
	CsrfSessionManager::new()
}

/// Creates a CSRF session manager with custom key
#[fixture]
fn csrf_manager_custom_key() -> CsrfSessionManager {
	CsrfSessionManager::with_key("custom_csrf_key".to_string())
}

// =============================================================================
// Happy Path Tests
// =============================================================================

#[rstest]
fn test_generate_token_creates_valid_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Act
	let token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Assert
	assert!(!token.is_empty(), "Token should not be empty");
	assert_eq!(
		token.len(),
		36,
		"Token should be a UUID (36 chars with hyphens)"
	);
	assert!(
		token.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
		"Token should contain only hex digits and hyphens"
	);
}

#[rstest]
fn test_get_token_returns_stored_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	let generated_token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act
	let retrieved_token = csrf_manager
		.get_token(&mut session)
		.expect("Get token should succeed")
		.expect("Token should exist");

	// Assert
	assert_eq!(generated_token, retrieved_token);
}

#[rstest]
fn test_validate_token_accepts_correct_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	let token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act
	let is_valid = csrf_manager
		.validate_token(&mut session, &token)
		.expect("Validation should succeed");

	// Assert
	assert!(is_valid, "Valid token should be accepted");
}

#[rstest]
fn test_rotate_token_generates_new_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	let old_token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act
	let new_token = csrf_manager
		.rotate_token(&mut session)
		.expect("Token rotation should succeed");

	// Assert
	assert_ne!(old_token, new_token, "Rotated token should be different");
	assert!(!new_token.is_empty(), "New token should not be empty");
}

#[rstest]
fn test_get_or_create_token_returns_existing(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	let first_token = csrf_manager
		.get_or_create_token(&mut session)
		.expect("Get or create should succeed");

	// Act
	let second_token = csrf_manager
		.get_or_create_token(&mut session)
		.expect("Get or create should succeed");

	// Assert
	assert_eq!(
		first_token, second_token,
		"Should return same token on second call"
	);
}

#[rstest]
fn test_clear_token_removes_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Verify token exists
	assert!(csrf_manager.get_token(&mut session).unwrap().is_some());

	// Act
	csrf_manager.clear_token(&mut session);

	// Assert
	let token = csrf_manager
		.get_token(&mut session)
		.expect("Get token should succeed");
	assert!(token.is_none(), "Token should be cleared");
}

// =============================================================================
// Error Path Tests
// =============================================================================

#[rstest]
fn test_validate_token_rejects_wrong_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act
	let is_valid = csrf_manager
		.validate_token(&mut session, "wrong-token-value")
		.expect("Validation should succeed");

	// Assert
	assert!(!is_valid, "Wrong token should be rejected");
}

#[rstest]
fn test_validate_token_rejects_empty_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act
	let is_valid = csrf_manager
		.validate_token(&mut session, "")
		.expect("Validation should succeed");

	// Assert
	assert!(!is_valid, "Empty token should be rejected");
}

#[rstest]
fn test_validate_token_fails_without_session_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Act - No token generated, just validate
	let is_valid = csrf_manager
		.validate_token(&mut session, "any-token")
		.expect("Validation should succeed");

	// Assert
	assert!(!is_valid, "Should reject when no token in session");
}

#[rstest]
fn test_get_token_returns_none_when_not_set(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Act
	let token = csrf_manager
		.get_token(&mut session)
		.expect("Get token should succeed");

	// Assert
	assert!(token.is_none(), "Should return None when no token set");
}

// =============================================================================
// State Transition Tests
// =============================================================================

#[rstest]
fn test_token_lifecycle_generate_validate_rotate_validate(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Phase 1: Generate
	let initial_token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Phase 2: Validate initial token
	assert!(
		csrf_manager
			.validate_token(&mut session, &initial_token)
			.unwrap(),
		"Initial token should be valid"
	);

	// Phase 3: Rotate
	let rotated_token = csrf_manager
		.rotate_token(&mut session)
		.expect("Token rotation should succeed");

	// Phase 4: Validate old token (should fail)
	assert!(
		!csrf_manager
			.validate_token(&mut session, &initial_token)
			.unwrap(),
		"Old token should be invalid after rotation"
	);

	// Phase 5: Validate new token (should succeed)
	assert!(
		csrf_manager
			.validate_token(&mut session, &rotated_token)
			.unwrap(),
		"New token should be valid after rotation"
	);
}

#[rstest]
fn test_token_lifecycle_generate_clear_generate(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Phase 1: Generate first token
	let first_token = csrf_manager
		.generate_token(&mut session)
		.expect("First generation should succeed");

	// Phase 2: Clear
	csrf_manager.clear_token(&mut session);
	assert!(csrf_manager.get_token(&mut session).unwrap().is_none());

	// Phase 3: Generate new token
	let second_token = csrf_manager
		.generate_token(&mut session)
		.expect("Second generation should succeed");

	// Assert: New token should be different
	assert_ne!(first_token, second_token, "New token should be different");
	assert!(
		csrf_manager
			.validate_token(&mut session, &second_token)
			.unwrap(),
		"New token should be valid"
	);
}

#[rstest]
fn test_multiple_rotations_create_unique_tokens(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Generate initial token
	let mut tokens = vec![
		csrf_manager
			.generate_token(&mut session)
			.expect("Initial generation should succeed"),
	];

	// Rotate 10 times
	for _ in 0..10 {
		let new_token = csrf_manager
			.rotate_token(&mut session)
			.expect("Rotation should succeed");

		// Each token should be unique
		assert!(
			!tokens.contains(&new_token),
			"Each rotated token should be unique"
		);
		tokens.push(new_token);
	}

	// Only the last token should be valid
	let last_token = tokens.last().unwrap();
	assert!(
		csrf_manager
			.validate_token(&mut session, last_token)
			.unwrap(),
		"Only last token should be valid"
	);

	// All previous tokens should be invalid
	for token in tokens.iter().take(tokens.len() - 1) {
		assert!(
			!csrf_manager.validate_token(&mut session, token).unwrap(),
			"Previous tokens should be invalid"
		);
	}
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[rstest]
fn test_custom_session_key_isolates_tokens(mut session: Session<InMemorySessionBackend>) {
	// Arrange - Two managers with different keys
	let manager1 = CsrfSessionManager::with_key("csrf_key_1".to_string());
	let manager2 = CsrfSessionManager::with_key("csrf_key_2".to_string());

	// Act - Generate tokens with each manager
	let token1 = manager1
		.generate_token(&mut session)
		.expect("Token1 generation should succeed");
	let token2 = manager2
		.generate_token(&mut session)
		.expect("Token2 generation should succeed");

	// Assert - Tokens should be independent
	assert_ne!(
		token1, token2,
		"Different keys should produce different tokens"
	);

	// Validate token1 with manager1
	assert!(
		manager1.validate_token(&mut session, &token1).unwrap(),
		"Token1 should be valid with manager1"
	);

	// Token1 should NOT be valid with manager2
	assert!(
		!manager2.validate_token(&mut session, &token1).unwrap(),
		"Token1 should not be valid with manager2"
	);
}

#[rstest]
fn test_token_with_special_characters_submitted(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	// Act - Try to validate with special characters
	let special_tokens = vec![
		"<script>alert('xss')</script>",
		"' OR '1'='1",
		"../../../etc/passwd",
		"\x00\x01\x02",
		"トークン",
		"🔐🔑",
	];

	// Assert - All should be rejected
	for special_token in special_tokens {
		let is_valid = csrf_manager
			.validate_token(&mut session, special_token)
			.expect("Validation should not error");
		assert!(
			!is_valid,
			"Special token '{}' should be rejected",
			special_token
		);
	}
}

#[rstest]
fn test_clear_token_is_idempotent(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Clear without generating (should not error)
	csrf_manager.clear_token(&mut session);
	csrf_manager.clear_token(&mut session);
	csrf_manager.clear_token(&mut session);

	// Token should still be None
	assert!(csrf_manager.get_token(&mut session).unwrap().is_none());

	// Generate and clear multiple times
	csrf_manager.generate_token(&mut session).unwrap();
	csrf_manager.clear_token(&mut session);
	csrf_manager.clear_token(&mut session);

	assert!(csrf_manager.get_token(&mut session).unwrap().is_none());
}

#[rstest]
fn test_token_regeneration_after_clear(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Arrange
	let original_token = csrf_manager
		.generate_token(&mut session)
		.expect("Token generation should succeed");

	csrf_manager.clear_token(&mut session);

	// Act
	let new_token = csrf_manager
		.get_or_create_token(&mut session)
		.expect("Get or create should succeed");

	// Assert
	assert_ne!(
		original_token, new_token,
		"New token should be different after clear"
	);
	assert!(
		csrf_manager
			.validate_token(&mut session, &new_token)
			.unwrap(),
		"New token should be valid"
	);
}

// =============================================================================
// Decision Table Tests
// =============================================================================

#[rstest]
#[case(true, true, true)] // Token exists + Token matches = Valid
#[case(true, false, false)] // Token exists + Token doesn't match = Invalid
#[case(false, true, false)] // No token + Any input = Invalid
#[case(false, false, false)] // No token + Wrong input = Invalid
fn test_validation_decision_table(
	#[case] token_exists: bool,
	#[case] token_matches: bool,
	#[case] expected_valid: bool,
) {
	// Arrange
	let backend = InMemorySessionBackend::new();
	let mut session = Session::new(backend);
	let csrf_manager = CsrfSessionManager::new();

	let submitted_token = if token_exists {
		let generated = csrf_manager
			.generate_token(&mut session)
			.expect("Token generation should succeed");
		if token_matches {
			generated
		} else {
			"different-token".to_string()
		}
	} else {
		"any-token".to_string()
	};

	// Act
	let is_valid = csrf_manager
		.validate_token(&mut session, &submitted_token)
		.expect("Validation should succeed");

	// Assert
	assert_eq!(
		is_valid, expected_valid,
		"Token validation for (exists={}, matches={}) should be {}",
		token_exists, token_matches, expected_valid
	);
}

// =============================================================================
// Use Case Tests
// =============================================================================

#[rstest]
fn test_use_case_form_submission_flow(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Step 1: User loads form page - generate or get token
	let form_token = csrf_manager
		.get_or_create_token(&mut session)
		.expect("Token should be available for form");

	// Step 2: User submits form with token
	let submitted_token = form_token.clone();
	let is_valid = csrf_manager
		.validate_token(&mut session, &submitted_token)
		.expect("Validation should succeed");

	assert!(is_valid, "Form submission should be valid");

	// Step 3: After successful action, rotate token (prevents replay)
	let new_token = csrf_manager
		.rotate_token(&mut session)
		.expect("Rotation should succeed");

	// Step 4: Replay attack with old token should fail
	let replay_valid = csrf_manager
		.validate_token(&mut session, &submitted_token)
		.expect("Validation should succeed");

	assert!(!replay_valid, "Replay attack should be prevented");
	assert_ne!(form_token, new_token, "New token should be different");
}

#[rstest]
fn test_use_case_login_token_rotation(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// Before login: Anonymous user has a token
	let pre_login_token = csrf_manager
		.generate_token(&mut session)
		.expect("Pre-login token should be generated");

	// User logs in - security best practice is to rotate the token
	let post_login_token = csrf_manager
		.rotate_token(&mut session)
		.expect("Post-login rotation should succeed");

	// Assert
	assert_ne!(
		pre_login_token, post_login_token,
		"Token should change after login"
	);
	assert!(
		!csrf_manager
			.validate_token(&mut session, &pre_login_token)
			.unwrap(),
		"Pre-login token should be invalid"
	);
	assert!(
		csrf_manager
			.validate_token(&mut session, &post_login_token)
			.unwrap(),
		"Post-login token should be valid"
	);
}

#[rstest]
fn test_use_case_logout_clears_token(
	mut session: Session<InMemorySessionBackend>,
	csrf_manager: CsrfSessionManager,
) {
	// User is logged in with a token
	let token = csrf_manager
		.generate_token(&mut session)
		.expect("Token should be generated");

	// User logs out - clear token
	csrf_manager.clear_token(&mut session);

	// Assert - token should be gone
	assert!(
		csrf_manager.get_token(&mut session).unwrap().is_none(),
		"Token should be cleared after logout"
	);
	assert!(
		!csrf_manager.validate_token(&mut session, &token).unwrap(),
		"Old token should be invalid after logout"
	);
}
