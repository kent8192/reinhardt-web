//! CSRF Attack Scenario Tests
//!
//! Comprehensive security testing for CSRF protection.
//! Tests attack scenarios from an attacker's perspective to verify
//! that the CSRF protection implementation correctly rejects malicious requests.
//!
//! Test Categories:
//! - Session Binding Attacks: 7 tests
//! - Replay Attacks: 6 tests
//! - Timestamp Expiry Attacks: 5 tests
//! - Cross-Origin Attacks: 6 tests
//! - Combined Attack Scenarios: 3 tests
//! - Parametric Variations: 3 tests
//!
//! Total: 30 tests

use hyper::header::{COOKIE, HeaderName, HeaderValue, ORIGIN, REFERER};
use reinhardt_http::Request;
use reinhardt_security::csrf::{
	RejectRequest, check_origin, check_referer, generate_token_hmac, generate_token_with_timestamp,
	should_rotate_token, verify_token_hmac, verify_token_with_timestamp,
};
use reinhardt_test::http::*;
use rstest::*;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Test Fixtures
// ============================================================================

#[fixture]
fn secret_key() -> Vec<u8> {
	b"test_secret_key_32_bytes_long!!".to_vec()
}

#[fixture]
fn session_id_alice() -> String {
	"alice_session_12345".to_string()
}

#[fixture]
fn session_id_bob() -> String {
	"bob_session_67890".to_string()
}

#[fixture]
fn allowed_origins() -> Vec<String> {
	vec!["https://example.com".to_string()]
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create HTTP request with CSRF token in both cookie and header
fn create_request_with_token(
	method: &str,
	uri: &str,
	cookie_token: &str,
	header_token: &str,
) -> Request {
	let mut request = create_test_request(method, uri, true);

	// Add cookie
	request.headers.insert(
		COOKIE,
		HeaderValue::from_str(&format!("csrftoken={}", cookie_token)).unwrap(),
	);

	// Add header
	request.headers.insert(
		HeaderName::from_static("x-csrftoken"),
		HeaderValue::from_str(header_token).unwrap(),
	);

	request
}

/// Create CSRF token with custom timestamp for testing expiry scenarios
fn create_token_with_custom_timestamp(secret: &[u8], session_id: &str, timestamp: u64) -> String {
	let message = format!("{}:{}", session_id, timestamp);
	let token = generate_token_hmac(secret, &message);
	format!("{}:{}", token, timestamp)
}

/// Create HTTP request with specific Origin header
fn create_request_with_origin(method: &str, uri: &str, origin: &str) -> Request {
	let mut request = create_test_request(method, uri, true);
	request
		.headers
		.insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
	request
}

/// Create HTTP request with specific Referer header
fn create_request_with_referer(method: &str, uri: &str, referer: &str) -> Request {
	let mut request = create_test_request(method, uri, true);
	request
		.headers
		.insert(REFERER, HeaderValue::from_str(referer).unwrap());
	request
}

/// Assert that a CSRF validation result is a rejection
fn assert_csrf_rejection(result: Result<(), RejectRequest>, expected_reason_contains: &str) {
	assert!(result.is_err(), "Expected CSRF rejection but got success");
	let err = result.unwrap_err();
	assert!(
		err.reason.contains(expected_reason_contains),
		"Expected reason to contain '{}', got '{}'",
		expected_reason_contains,
		err.reason
	);
}

/// Get current timestamp in seconds since UNIX epoch
fn get_current_timestamp() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs()
}

// ============================================================================
// Session Binding Attacks (7 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn test_attack_other_user_token(
	secret_key: Vec<u8>,
	session_id_alice: String,
	session_id_bob: String,
) {
	// Test: Alice's token should not work for Bob's session
	// This verifies session binding - tokens are tied to specific sessions

	// Alice generates her token
	let alice_token = generate_token_hmac(&secret_key, &session_id_alice);

	// Attack: Bob tries to use Alice's token
	let result = verify_token_hmac(&alice_token, &secret_key, &session_id_bob);

	// Attack should fail - token is bound to Alice's session
	assert!(!result, "Token from different session should not be valid");
}

#[rstest]
#[tokio::test]
async fn test_attack_token_session_mismatch(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Token generated for one session ID should not work with a different session ID
	// This is the core session binding verification

	let token = generate_token_hmac(&secret_key, &session_id_alice);

	// Attack: Try to verify with a completely different session ID
	let wrong_session_id = "completely_different_session_xyz";
	let result = verify_token_hmac(&token, &secret_key, wrong_session_id);

	// Attack should fail
	assert!(
		!result,
		"Token should not be valid with mismatched session ID"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_session_fixation_with_csrf(
	secret_key: Vec<u8>,
	session_id_alice: String,
	session_id_bob: String,
) {
	// Test: Session fixation attack scenario
	// Attacker fixes a session ID and tries to use CSRF token from that session

	// Attacker fixes Bob's session and generates a token
	let fixed_token = generate_token_hmac(&secret_key, &session_id_bob);

	// Victim (Alice) has her own session
	// Attack: Try to use the fixed token in Alice's session
	let result = verify_token_hmac(&fixed_token, &secret_key, &session_id_alice);

	// Attack should fail - even with session fixation, CSRF token binding prevents misuse
	assert!(
		!result,
		"Session fixation attack should be prevented by token binding"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_token_reuse_across_sessions(secret_key: Vec<u8>) {
	// Test: Token should not be reusable across multiple different sessions

	let original_session = "session_original";
	let token = generate_token_hmac(&secret_key, original_session);

	// Try to reuse token in multiple different sessions
	let sessions = vec![
		"session_1",
		"session_2",
		"session_3",
		"completely_different",
	];

	for session in sessions {
		let result = verify_token_hmac(&token, &secret_key, session);
		assert!(!result, "Token should not be valid in session: {}", session);
	}
}

#[rstest]
#[tokio::test]
async fn test_attack_null_session_token(secret_key: Vec<u8>) {
	// Test: Token generated with empty session ID should not be usable

	let empty_session = "";
	let token = generate_token_hmac(&secret_key, empty_session);

	// Try to use with a real session ID
	let real_session = "real_session_id";
	let result = verify_token_hmac(&token, &secret_key, real_session);

	// Attack should fail
	assert!(!result, "Null session token should not be valid");
}

#[rstest]
#[case(
	"session_with_very_long_id_that_exceeds_normal_length_expectations_abcdefghijklmnopqrstuvwxyz0123456789"
)]
#[case("session\nwith\nnewlines")]
#[case("session\twith\ttabs")]
#[case("session with spaces")]
#[tokio::test]
async fn test_attack_edge_case_session_ids(secret_key: Vec<u8>, #[case] edge_case_session: &str) {
	// Test: Tokens with edge case session IDs should not cross-validate

	let token = generate_token_hmac(&secret_key, edge_case_session);
	let normal_session = "normal_session_id";

	// Try to use edge case token with normal session
	let result = verify_token_hmac(&token, &secret_key, normal_session);
	assert!(!result, "Edge case session token should not be valid");

	// Try to use normal token with edge case session
	let normal_token = generate_token_hmac(&secret_key, normal_session);
	let result2 = verify_token_hmac(&normal_token, &secret_key, edge_case_session);
	assert!(
		!result2,
		"Normal token should not be valid for edge case session"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_case_sensitive_session_id(secret_key: Vec<u8>) {
	// Test: Session IDs should be case-sensitive for security

	let lowercase_session = "session_abc";
	let uppercase_session = "SESSION_ABC";

	let token = generate_token_hmac(&secret_key, lowercase_session);

	// Try to use lowercase token with uppercase session
	let result = verify_token_hmac(&token, &secret_key, uppercase_session);

	// Attack should fail - case sensitivity is important for security
	assert!(
		!result,
		"Case-different session IDs should not share tokens"
	);
}

// ============================================================================
// Replay Attacks (6 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn test_attack_replay_rotated_token(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: After token rotation, old token should trigger rotation detection
	// This prevents replay attacks using previously valid tokens

	// Generate initial token with timestamp
	let old_token = generate_token_with_timestamp(&secret_key, &session_id_alice);

	// Extract timestamp from old token
	let parts: Vec<&str> = old_token.split(':').collect();
	let old_timestamp: u64 = parts[1].parse().unwrap();

	// Simulate time passing (2 hours = 7200 seconds)
	let current_timestamp = old_timestamp + 7200;

	// Check if rotation is required (rotation_interval = 3600 = 1 hour)
	let should_rotate = should_rotate_token(old_timestamp, current_timestamp, Some(3600));

	// Old token should trigger rotation
	assert!(
		should_rotate,
		"Old token should trigger rotation after interval"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_replay_timestamped_token(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Replaying a timestamped token should be detectable

	// Create old timestamped token (1 hour ago)
	let old_timestamp = get_current_timestamp() - 3600;
	let old_token =
		create_token_with_custom_timestamp(&secret_key, &session_id_alice, old_timestamp);

	// Verify that the token structure is valid but triggers rotation check
	let verify_result = verify_token_with_timestamp(&old_token, &secret_key, &session_id_alice);

	// Token should verify structurally
	assert!(
		verify_result.is_ok(),
		"Old token should still verify structurally"
	);

	// But should trigger rotation
	let extracted_timestamp = verify_result.unwrap();
	let current_timestamp = get_current_timestamp();
	let should_rotate = should_rotate_token(extracted_timestamp, current_timestamp, Some(1800)); // 30 min interval

	assert!(should_rotate, "Replayed old token should trigger rotation");
}

#[rstest]
#[tokio::test]
async fn test_attack_concurrent_token_reuse(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Reusing the same token in concurrent requests should still verify correctly
	// (CSRF tokens can be reused within their validity period, but should be tied to session)

	let token = generate_token_hmac(&secret_key, &session_id_alice);

	// Simulate concurrent verifications with correct session
	let result1 = verify_token_hmac(&token, &secret_key, &session_id_alice);
	let result2 = verify_token_hmac(&token, &secret_key, &session_id_alice);
	let result3 = verify_token_hmac(&token, &secret_key, &session_id_alice);

	// All should succeed with the correct session
	assert!(result1, "First verification should succeed");
	assert!(result2, "Second verification should succeed");
	assert!(result3, "Third verification should succeed");

	// But should fail with different session
	let wrong_session = "wrong_session";
	let result4 = verify_token_hmac(&token, &secret_key, wrong_session);
	assert!(!result4, "Verification with wrong session should fail");
}

#[rstest]
#[tokio::test]
async fn test_attack_replay_after_logout(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Token from before logout should not be usable after session ID changes

	// Generate token before "logout"
	let token_before_logout = generate_token_hmac(&secret_key, &session_id_alice);

	// After logout, session ID changes
	let new_session_after_logout = "alice_new_session_after_logout";

	// Attack: Try to replay old token with new session
	let result = verify_token_hmac(&token_before_logout, &secret_key, new_session_after_logout);

	// Attack should fail - session has changed
	assert!(
		!result,
		"Token from before logout should not work with new session"
	);
}

#[rstest]
#[case(0)] // Timestamp = 0 (epoch)
#[case(u64::MAX)] // Maximum timestamp
#[case(1)] // Very early timestamp
#[tokio::test]
async fn test_attack_edge_case_timestamps(
	secret_key: Vec<u8>,
	session_id_alice: String,
	#[case] edge_timestamp: u64,
) {
	// Test: Edge case timestamps should be handled correctly

	let token = create_token_with_custom_timestamp(&secret_key, &session_id_alice, edge_timestamp);

	// Should be able to parse and verify structure
	let result = verify_token_with_timestamp(&token, &secret_key, &session_id_alice);

	// Should succeed in parsing
	assert!(result.is_ok(), "Edge case timestamp should parse correctly");

	let extracted_timestamp = result.unwrap();
	assert_eq!(
		extracted_timestamp, edge_timestamp,
		"Extracted timestamp should match"
	);
}

#[rstest]
#[case("token_without_colon")]
#[case("token:with:too:many:colons")]
#[case("token:not_a_number")]
#[case("token:12.34")] // Float instead of integer
#[tokio::test]
async fn test_attack_malformed_timestamp_token(
	secret_key: Vec<u8>,
	session_id_alice: String,
	#[case] malformed_token: &str,
) {
	// Test: Malformed timestamp tokens should be rejected

	let result = verify_token_with_timestamp(malformed_token, &secret_key, &session_id_alice);

	// Should fail to parse
	assert!(result.is_err(), "Malformed token should be rejected");
}

// ============================================================================
// Timestamp Expiry Attacks (5 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn test_attack_expired_timestamp_token(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Token with expired timestamp should trigger rotation

	// Create token with old timestamp (2 hours ago)
	let old_timestamp = get_current_timestamp() - 7200;
	let token = create_token_with_custom_timestamp(&secret_key, &session_id_alice, old_timestamp);

	// Verify the token
	let verify_result = verify_token_with_timestamp(&token, &secret_key, &session_id_alice);
	assert!(verify_result.is_ok(), "Token should verify structurally");

	let extracted_timestamp = verify_result.unwrap();
	let current_timestamp = get_current_timestamp();

	// Check if rotation is required (rotation_interval = 3600 = 1 hour)
	let should_rotate = should_rotate_token(extracted_timestamp, current_timestamp, Some(3600));

	// Should trigger rotation
	assert!(
		should_rotate,
		"Expired token should trigger rotation requirement"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_future_timestamp_token(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Token with future timestamp should be detectable

	// Create token with future timestamp (1 hour ahead)
	let future_timestamp = get_current_timestamp() + 3600;
	let token =
		create_token_with_custom_timestamp(&secret_key, &session_id_alice, future_timestamp);

	// Verify the token
	let verify_result = verify_token_with_timestamp(&token, &secret_key, &session_id_alice);

	// Should still verify structurally (timestamp validation is separate)
	assert!(
		verify_result.is_ok(),
		"Future token should verify structurally"
	);

	let extracted_timestamp = verify_result.unwrap();
	let current_timestamp = get_current_timestamp();

	// Future timestamp should be detectable
	assert!(
		extracted_timestamp > current_timestamp,
		"Future timestamp should be detectable"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_missing_timestamp_in_token(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Token without timestamp should be rejected by timestamp verification

	// Create token without timestamp
	let token_without_timestamp = generate_token_hmac(&secret_key, &session_id_alice);

	// Try to verify as timestamped token
	let result =
		verify_token_with_timestamp(&token_without_timestamp, &secret_key, &session_id_alice);

	// Should fail - missing timestamp
	assert!(
		result.is_err(),
		"Token without timestamp should be rejected"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_rotation_boundary_timestamp(secret_key: Vec<u8>, session_id_alice: String) {
	// Test: Timestamps around rotation boundary should be handled correctly

	let base_timestamp = get_current_timestamp();
	let rotation_interval = 3600; // 1 hour

	// Test just before rotation boundary
	let before_boundary = base_timestamp + rotation_interval - 1;
	let should_rotate_before =
		should_rotate_token(base_timestamp, before_boundary, Some(rotation_interval));
	assert!(
		!should_rotate_before,
		"Should not rotate just before boundary"
	);

	// Test exactly at rotation boundary
	let at_boundary = base_timestamp + rotation_interval;
	let should_rotate_at =
		should_rotate_token(base_timestamp, at_boundary, Some(rotation_interval));
	assert!(should_rotate_at, "Should rotate exactly at boundary");

	// Test just after rotation boundary
	let after_boundary = base_timestamp + rotation_interval + 1;
	let should_rotate_after =
		should_rotate_token(base_timestamp, after_boundary, Some(rotation_interval));
	assert!(should_rotate_after, "Should rotate just after boundary");
}

#[rstest]
#[case(1)] // 1 second old
#[case(60)] // 1 minute old
#[case(3600)] // 1 hour old
#[case(86400)] // 1 day old
#[tokio::test]
async fn test_attack_various_timestamp_ages(
	secret_key: Vec<u8>,
	session_id_alice: String,
	#[case] age_seconds: u64,
) {
	// Test: Tokens of various ages should be handled correctly

	let old_timestamp = get_current_timestamp() - age_seconds;
	let token = create_token_with_custom_timestamp(&secret_key, &session_id_alice, old_timestamp);

	// Verify the token
	let result = verify_token_with_timestamp(&token, &secret_key, &session_id_alice);
	assert!(result.is_ok(), "Token should verify structurally");

	// Check rotation with 1-hour interval
	let current_timestamp = get_current_timestamp();
	let should_rotate = should_rotate_token(old_timestamp, current_timestamp, Some(3600));

	if age_seconds >= 3600 {
		assert!(
			should_rotate,
			"Token older than 1 hour should trigger rotation"
		);
	} else {
		assert!(
			!should_rotate,
			"Token younger than 1 hour should not trigger rotation"
		);
	}
}

// ============================================================================
// Cross-Origin Attacks (6 tests)
// ============================================================================

#[rstest]
#[case("https://evil.com")]
#[case("http://example.com")] // Wrong protocol
#[case("https://example.com.evil.com")] // Subdomain spoofing
#[case("https://examplexcom")] // Typosquatting
#[tokio::test]
async fn test_attack_different_origin_header(
	allowed_origins: Vec<String>,
	#[case] malicious_origin: &str,
) {
	// Test: Requests with malicious Origin headers should be rejected

	let result = check_origin(malicious_origin, &allowed_origins);

	// Attack should be rejected
	assert!(
		result.is_err(),
		"Malicious origin '{}' should be rejected",
		malicious_origin
	);
}

#[rstest]
#[case("https://evil.com/page")]
#[case("http://example.com/page")] // Wrong protocol
#[case("https://example.com.evil.com/page")] // Subdomain spoofing
#[tokio::test]
async fn test_attack_different_referer_header(
	allowed_origins: Vec<String>,
	#[case] malicious_referer: &str,
) {
	// Test: Requests with malicious Referer headers should be rejected

	let result = check_referer(Some(malicious_referer), &allowed_origins, true);

	// Attack should be rejected
	assert!(
		result.is_err(),
		"Malicious referer '{}' should be rejected",
		malicious_referer
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_missing_referer_header(allowed_origins: Vec<String>) {
	// Test: Secure requests without Referer header should be rejected

	let result = check_referer(None, &allowed_origins, true);

	// Attack should be rejected (missing Referer on secure request)
	assert!(result.is_err(), "Missing Referer should be rejected");
}

#[rstest]
#[tokio::test]
async fn test_attack_https_to_http_downgrade() {
	// Test: HTTPS site should reject HTTP referers (protocol downgrade attack)

	let allowed_origins = vec!["https://example.com".to_string()];
	let http_referer = "http://example.com/page"; // Downgraded to HTTP

	let result = check_referer(Some(http_referer), &allowed_origins, true);

	// Attack should be rejected
	assert!(
		result.is_err(),
		"HTTP referer to HTTPS site should be rejected"
	);
}

#[rstest]
#[case("https://example.com:8080/page")] // Different port
#[case("https://example.com/page?evil=param")] // With query params
#[case("https://example.com/../../etc/passwd")] // Path traversal attempt
#[tokio::test]
async fn test_attack_origin_variations(
	allowed_origins: Vec<String>,
	#[case] variation_origin: &str,
) {
	// Test: Origin variations should be handled correctly

	// For port variations, they should be rejected unless explicitly allowed
	let result = check_referer(Some(variation_origin), &allowed_origins, true);

	// Most variations should be rejected (unless exact match)
	if variation_origin.starts_with("https://example.com:") {
		// Port mismatch should be rejected
		assert!(
			result.is_err(),
			"Origin with different port should be rejected"
		);
	}
	// Query params and path should be okay if origin matches
}

#[rstest]
#[case("https://examplе.com")] // Cyrillic 'е' instead of Latin 'e'
#[case("https://ехample.com")] // Cyrillic 'х' instead of Latin 'x'
#[tokio::test]
async fn test_attack_idn_spoofing(allowed_origins: Vec<String>, #[case] spoofed_origin: &str) {
	// Test: IDN homograph attacks should be prevented

	let result = check_origin(spoofed_origin, &allowed_origins);

	// Spoofed origin should not match
	assert!(
		result.is_err(),
		"IDN spoofed origin '{}' should be rejected",
		spoofed_origin
	);
}

// ============================================================================
// Combined Attack Scenarios (3 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn test_attack_session_hijack_plus_csrf(
	secret_key: Vec<u8>,
	session_id_alice: String,
	session_id_bob: String,
) {
	// Test: Even if attacker hijacks session, CSRF token binding prevents misuse
	// Scenario: Attacker has Bob's token, tries to use in hijacked Alice's session

	let bob_token = generate_token_hmac(&secret_key, &session_id_bob);

	// Attack: Use Bob's token in Alice's hijacked session
	let result = verify_token_hmac(&bob_token, &secret_key, &session_id_alice);

	// Attack should fail - CSRF token is still bound to Bob's session
	assert!(
		!result,
		"Session hijacking should not allow CSRF token reuse"
	);
}

#[rstest]
#[tokio::test]
async fn test_attack_cross_origin_with_token_reuse(
	secret_key: Vec<u8>,
	session_id_alice: String,
	allowed_origins: Vec<String>,
) {
	// Test: Cross-origin request with replayed old token should be rejected on multiple fronts

	// Create old timestamped token
	let old_timestamp = get_current_timestamp() - 7200; // 2 hours ago
	let old_token =
		create_token_with_custom_timestamp(&secret_key, &session_id_alice, old_timestamp);

	// Check 1: Cross-origin check
	let malicious_origin = "https://evil.com";
	let origin_check = check_origin(malicious_origin, &allowed_origins);
	assert!(origin_check.is_err(), "Origin check should fail");

	// Check 2: Token replay check
	let verify_result = verify_token_with_timestamp(&old_token, &secret_key, &session_id_alice);
	assert!(verify_result.is_ok(), "Token structure should verify");

	let extracted_timestamp = verify_result.unwrap();
	let current_timestamp = get_current_timestamp();
	let should_rotate = should_rotate_token(extracted_timestamp, current_timestamp, Some(3600));
	assert!(should_rotate, "Old token should trigger rotation");

	// Both checks fail - defense in depth
}

#[rstest]
#[tokio::test]
async fn test_attack_triple_combination(
	secret_key: Vec<u8>,
	session_id_alice: String,
	session_id_bob: String,
	allowed_origins: Vec<String>,
) {
	// Test: Triple attack - wrong session + expired token + cross-origin
	// All three defenses should catch this

	// Create old token for Bob
	let old_timestamp = get_current_timestamp() - 7200;
	let bob_old_token =
		create_token_with_custom_timestamp(&secret_key, &session_id_bob, old_timestamp);

	// Attack 1: Wrong session (Bob's token in Alice's session)
	let verify_result = verify_token_with_timestamp(&bob_old_token, &secret_key, &session_id_alice);
	assert!(
		verify_result.is_err(),
		"Wrong session should fail verification"
	);

	// Attack 2: Expired token check
	let verify_result_correct_session =
		verify_token_with_timestamp(&bob_old_token, &secret_key, &session_id_bob);
	assert!(
		verify_result_correct_session.is_ok(),
		"Should verify with correct session"
	);
	let extracted_timestamp = verify_result_correct_session.unwrap();
	let should_rotate =
		should_rotate_token(extracted_timestamp, get_current_timestamp(), Some(3600));
	assert!(should_rotate, "Old token should trigger rotation");

	// Attack 3: Cross-origin check
	let origin_check = check_origin("https://evil.com", &allowed_origins);
	assert!(origin_check.is_err(), "Cross-origin should be rejected");

	// All three defenses work independently
}

// ============================================================================
// Parametric Variations (3 tests)
// ============================================================================

#[rstest]
#[case(b"very_short_key")]
#[case(b"exactly_32_bytes_long_secret_k!")]
#[case(b"very_long_secret_key_that_exceeds_typical_length_for_testing_purposes_abcdefghijklmnopqrstuvwxyz")]
#[tokio::test]
async fn test_attack_various_secret_lengths(
	session_id_alice: String,
	session_id_bob: String,
	#[case] test_secret: &[u8],
) {
	// Test: Different secret key lengths should still maintain session binding

	let alice_token = generate_token_hmac(test_secret, &session_id_alice);

	// Should work with correct session
	let correct_result = verify_token_hmac(&alice_token, test_secret, &session_id_alice);
	assert!(correct_result, "Should verify with correct session");

	// Should fail with wrong session
	let wrong_result = verify_token_hmac(&alice_token, test_secret, &session_id_bob);
	assert!(!wrong_result, "Should fail with wrong session");
}

#[rstest]
#[case(None)] // No rotation
#[case(Some(60))] // 1 minute
#[case(Some(1800))] // 30 minutes
#[case(Some(3600))] // 1 hour
#[case(Some(86400))] // 1 day
#[tokio::test]
async fn test_attack_various_rotation_intervals(
	secret_key: Vec<u8>,
	session_id_alice: String,
	#[case] rotation_interval: Option<u64>,
) {
	// Test: Different rotation intervals should be respected

	let old_timestamp = get_current_timestamp() - 7200; // 2 hours ago
	let current_timestamp = get_current_timestamp();

	let should_rotate = should_rotate_token(old_timestamp, current_timestamp, rotation_interval);

	match rotation_interval {
		None => assert!(
			!should_rotate,
			"No interval should mean no rotation trigger"
		),
		Some(interval) => {
			if 7200 >= interval {
				assert!(should_rotate, "Should rotate when age exceeds interval");
			} else {
				assert!(
					!should_rotate,
					"Should not rotate when age is less than interval"
				);
			}
		}
	}
}

#[rstest]
#[tokio::test]
async fn test_attack_empty_and_special_origins() {
	// Test: Empty and special origin values should be handled correctly

	let allowed_origins = vec!["https://example.com".to_string()];

	// Empty origin
	let empty_result = check_origin("", &allowed_origins);
	assert!(empty_result.is_err(), "Empty origin should be rejected");

	// Null origin (browser sends "null" as string for some cases)
	let null_result = check_origin("null", &allowed_origins);
	assert!(null_result.is_err(), "Null origin should be rejected");

	// Data URI
	let data_uri_result = check_origin("data:text/html", &allowed_origins);
	assert!(
		data_uri_result.is_err(),
		"Data URI origin should be rejected"
	);

	// File URI
	let file_uri_result = check_origin("file:///", &allowed_origins);
	assert!(
		file_uri_result.is_err(),
		"File URI origin should be rejected"
	);
}
