//! JWKS cache integration tests

use reinhardt_auth::social::oidc::{Jwk, JwkSet, JwksCache};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rstest::*;

#[test]
fn test_jwk_set_structure() {
	// Arrange
	let jwk = Jwk {
		kty: "RSA".to_string(),
		kid: Some("test_key_id".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("test_modulus".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	let jwks = JwkSet {
		keys: vec![jwk.clone()],
	};

	// Assert
	assert_eq!(jwks.keys.len(), 1);
	assert_eq!(jwks.keys[0].kid, Some("test_key_id".to_string()));
}

#[test]
fn test_jwk_retrieve_by_kid() {
	// Arrange
	let jwk1 = Jwk {
		kty: "RSA".to_string(),
		kid: Some("key1".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("modulus1".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	let jwk2 = Jwk {
		kty: "RSA".to_string(),
		kid: Some("key2".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("modulus2".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	let jwks = JwkSet {
		keys: vec![jwk1.clone(), jwk2],
	};

	// Act
	let key = jwks.get_key("key1");

	// Assert
	assert!(key.is_some());
	assert_eq!(key.unwrap().kid, Some("key1".to_string()));
}

#[test]
fn test_jwk_missing_key() {
	// Arrange
	let jwks = JwkSet { keys: vec![] };

	// Act
	let key = jwks.get_key("nonexistent");

	// Assert
	assert!(key.is_none());
}

#[tokio::test]
async fn test_jwks_cache_store_and_retrieve() {
	// Arrange
	let cache = JwksCache::new();
	let jwk = Jwk {
		kty: "RSA".to_string(),
		kid: Some("test_key".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("modulus".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	let jwks = JwkSet {
		keys: vec![jwk.clone()],
	};

	// Act - Store JWKS
	cache.set("https://example.com/jwks", jwks).await;

	// Retrieve JWKS
	let retrieved = cache.get("https://example.com/jwks").await;

	// Assert
	assert!(retrieved.is_some());
	let retrieved_jwks = retrieved.unwrap();
	assert_eq!(retrieved_jwks.keys.len(), 1);
	assert_eq!(retrieved_jwks.keys[0].kid, Some("test_key".to_string()));
}

#[tokio::test]
async fn test_jwks_cache_miss() {
	// Arrange
	let cache = JwksCache::new();

	// Act
	let result = cache.get("https://example.com/jwks").await;

	// Assert
	assert!(result.is_none());
}

#[test]
fn test_jwk_rsa_key_support() {
	// Arrange
	let jwk = Jwk {
		kty: "RSA".to_string(),
		kid: Some("test_key".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("test_modulus".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	// Assert
	assert_eq!(jwk.kty, "RSA");
	assert!(jwk.n.is_some());
	assert!(jwk.e.is_some());
	assert_eq!(jwk.alg, Some("RS256".to_string()));
}

#[test]
fn test_jwk_ec_key_support() {
	// Arrange
	let jwk = Jwk {
		kty: "EC".to_string(),
		kid: Some("test_key".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("ES256".to_string()),
		n: None,
		e: None,
		crv: Some("P-256".to_string()),
		x: Some("test_x".to_string()),
		y: Some("test_y".to_string()),
	};

	// Assert
	assert_eq!(jwk.kty, "EC");
	assert!(jwk.x.is_some());
	assert!(jwk.y.is_some());
	assert!(jwk.crv.is_some());
}

#[test]
fn test_jwks_serialization() {
	// Arrange
	let jwk = Jwk {
		kty: "RSA".to_string(),
		kid: Some("test_key".to_string()),
		use_: Some("sig".to_string()),
		alg: Some("RS256".to_string()),
		n: Some("modulus".to_string()),
		e: Some("AQAB".to_string()),
		crv: None,
		x: None,
		y: None,
	};

	let jwks = JwkSet {
		keys: vec![jwk],
	};

	// Act
	let json = serde_json::to_string(&jwks).unwrap();

	// Assert
	assert!(json.contains("\"kty\":\"RSA\""));
	assert!(json.contains("\"kid\":\"test_key\""));
}

#[test]
fn test_jwks_deserialization() {
	// Arrange
	let json = r#"{
		"keys": [{
			"kty": "RSA",
			"kid": "test_key",
			"use": "sig",
			"alg": "RS256",
			"n": "test_modulus",
			"e": "AQAB"
		}]
	}"#;

	// Act
	let jwks: JwkSet = serde_json::from_str(json).unwrap();

	// Assert
	assert_eq!(jwks.keys.len(), 1);
	assert_eq!(jwks.keys[0].kid, Some("test_key".to_string()));
}
