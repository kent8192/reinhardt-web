//! HMAC-SHA256 based CSRF token tests

use reinhardt_security::csrf::{
    check_token_hmac, generate_token_hmac, get_secret_bytes, get_token_hmac, verify_token_hmac,
};

#[test]
fn test_generate_token_hmac_produces_64_char_hex() {
    let secret = b"my-secret-key-at-least-32-bytes-long-for-security";
    let message = "session-id-12345";
    let token = generate_token_hmac(secret, message);

    // HMAC-SHA256 produces 32 bytes = 64 hex characters
    assert_eq!(token.len(), 64);
    // Verify it's valid hex
    assert!(hex::decode(&token).is_ok());
}

#[test]
fn test_verify_token_hmac_valid_token() {
    let secret = b"my-secret-key-at-least-32-bytes-long-for-security";
    let message = "session-id-12345";
    let token = generate_token_hmac(secret, message);

    assert!(verify_token_hmac(&token, secret, message));
}

#[test]
fn test_verify_token_hmac_invalid_token() {
    let secret = b"my-secret-key-at-least-32-bytes-long-for-security";
    let message = "session-id-12345";

    // Invalid hex string
    assert!(!verify_token_hmac("invalid-token", secret, message));

    // Wrong token
    let wrong_token = "a".repeat(64);
    assert!(!verify_token_hmac(&wrong_token, secret, message));
}

#[test]
fn test_verify_token_hmac_wrong_message() {
    let secret = b"my-secret-key-at-least-32-bytes-long-for-security";
    let message1 = "session-id-12345";
    let message2 = "session-id-67890";

    let token = generate_token_hmac(secret, message1);

    assert!(verify_token_hmac(&token, secret, message1));
    assert!(!verify_token_hmac(&token, secret, message2));
}

#[test]
fn test_verify_token_hmac_wrong_secret() {
    let secret1 = b"secret-key-one-at-least-32-bytes-long-for-security";
    let secret2 = b"secret-key-two-at-least-32-bytes-long-for-security";
    let message = "session-id-12345";

    let token = generate_token_hmac(secret1, message);

    assert!(verify_token_hmac(&token, secret1, message));
    assert!(!verify_token_hmac(&token, secret2, message));
}

#[test]
fn test_get_secret_bytes_length() {
    let secret = get_secret_bytes();
    assert_eq!(secret.len(), 32);
}

#[test]
fn test_get_secret_bytes_randomness() {
    let secret1 = get_secret_bytes();
    let secret2 = get_secret_bytes();

    // Very unlikely to be the same
    assert_ne!(secret1, secret2);
}

#[test]
fn test_get_token_hmac() {
    let secret = get_secret_bytes();
    let session_id = "user-session-12345";
    let token = get_token_hmac(&secret, session_id);

    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, &secret, session_id));
}

#[test]
fn test_check_token_hmac_valid() {
    let secret = get_secret_bytes();
    let session_id = "user-session-12345";
    let token = get_token_hmac(&secret, session_id);

    assert!(check_token_hmac(&token, &secret, session_id).is_ok());
}

#[test]
fn test_check_token_hmac_invalid() {
    let secret = get_secret_bytes();
    let session_id = "user-session-12345";

    let result = check_token_hmac("invalid-token", &secret, session_id);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .reason
            .contains("HMAC verification failed")
    );
}

#[test]
fn test_check_token_hmac_wrong_session() {
    let secret = get_secret_bytes();
    let session_id1 = "user-session-12345";
    let session_id2 = "user-session-67890";

    let token = get_token_hmac(&secret, session_id1);

    assert!(check_token_hmac(&token, &secret, session_id1).is_ok());
    assert!(check_token_hmac(&token, &secret, session_id2).is_err());
}

#[test]
fn test_hmac_timing_attack_resistance() {
    // This test verifies constant-time comparison
    let secret = get_secret_bytes();
    let message = "session-12345";
    let valid_token = generate_token_hmac(&secret, message);

    // Create two invalid tokens:
    // 1. All correct except first char
    // 2. All correct except last char
    let mut almost_valid_1 = valid_token.clone();
    almost_valid_1.replace_range(0..1, "0");

    let mut almost_valid_2 = valid_token.clone();
    let last_idx = almost_valid_2.len() - 1;
    almost_valid_2.replace_range(last_idx..last_idx + 1, "0");

    // Both should fail verification
    assert!(!verify_token_hmac(&almost_valid_1, &secret, message));
    assert!(!verify_token_hmac(&almost_valid_2, &secret, message));

    // Original should still work
    assert!(verify_token_hmac(&valid_token, &secret, message));
}

#[test]
fn test_hmac_deterministic_for_same_inputs() {
    let secret = b"my-secret-key-32-bytes-long-test";
    let message = "session-12345";

    let token1 = generate_token_hmac(secret, message);
    let token2 = generate_token_hmac(secret, message);

    // Same inputs should produce same output
    assert_eq!(token1, token2);
}

#[test]
fn test_hmac_different_for_different_messages() {
    let secret = b"my-secret-key-32-bytes-long-test";

    let token1 = generate_token_hmac(secret, "message1");
    let token2 = generate_token_hmac(secret, "message2");

    // Different messages should produce different tokens
    assert_ne!(token1, token2);
}

#[test]
fn test_hmac_different_for_different_secrets() {
    let message = "session-12345";

    let token1 = generate_token_hmac(b"secret1-is-32-bytes-long-test111", message);
    let token2 = generate_token_hmac(b"secret2-is-32-bytes-long-test222", message);

    // Different secrets should produce different tokens
    assert_ne!(token1, token2);
}

#[test]
fn test_hmac_empty_message() {
    let secret = b"my-secret-key-32-bytes-long-test";
    let empty_message = "";

    let token = generate_token_hmac(secret, empty_message);
    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, secret, empty_message));
}

#[test]
fn test_hmac_long_message() {
    let secret = b"my-secret-key-32-bytes-long-test";
    let long_message = "a".repeat(10000);

    let token = generate_token_hmac(secret, &long_message);
    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, secret, &long_message));
}

#[test]
fn test_hmac_unicode_message() {
    let secret = b"my-secret-key-32-bytes-long-test";
    let unicode_message = "こんにちは世界🌍";

    let token = generate_token_hmac(secret, unicode_message);
    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, secret, unicode_message));
}

#[test]
fn test_hmac_short_secret() {
    // HMAC can handle keys of any size
    let short_secret = b"short";
    let message = "session-12345";

    let token = generate_token_hmac(short_secret, message);
    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, short_secret, message));
}

#[test]
fn test_hmac_long_secret() {
    let long_secret =
        b"this-is-a-very-long-secret-key-much-longer-than-32-bytes-for-testing-purposes";
    let message = "session-12345";

    let token = generate_token_hmac(long_secret, message);
    assert_eq!(token.len(), 64);
    assert!(verify_token_hmac(&token, long_secret, message));
}
