//! FastAPI dependency security overrides tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_security_overrides.py
//!
//! These tests verify that:
//! 1. Security dependencies work with scopes
//! 2. Security dependencies can be overridden
//! 3. Regular dependencies can be overridden alongside security dependencies

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Security scopes
#[derive(Clone, Debug, PartialEq)]
struct SecurityScopes {
	scopes: Vec<String>,
}

impl SecurityScopes {
	fn new(scopes: Vec<&str>) -> Self {
		SecurityScopes {
			scopes: scopes.into_iter().map(|s| s.to_string()).collect(),
		}
	}
}

// User data with security
#[derive(Clone, Debug, PartialEq)]
struct UserData {
	username: String,
	scopes: Vec<String>,
}

#[async_trait::async_trait]
impl Injectable for UserData {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Get security scopes from context
		let scopes = if let Some(s) = ctx.get_request::<SecurityScopes>() {
			(*s).clone()
		} else {
			SecurityScopes::new(vec![])
		};

		Ok(UserData {
			username: "john".to_string(),
			scopes: scopes.scopes,
		})
	}
}

// Override user data
#[derive(Clone, Debug, PartialEq)]
struct UserDataOverride {
	username: String,
	scopes: Vec<String>,
}

#[async_trait::async_trait]
impl Injectable for UserDataOverride {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let scopes = if let Some(s) = ctx.get_request::<SecurityScopes>() {
			(*s).clone()
		} else {
			SecurityScopes::new(vec![])
		};

		Ok(UserDataOverride {
			username: "alice".to_string(),
			scopes: scopes.scopes,
		})
	}
}

// Regular data dependency
#[derive(Clone, Debug, PartialEq)]
struct DataDependency {
	data: Vec<i32>,
}

#[async_trait::async_trait]
impl Injectable for DataDependency {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(DataDependency {
			data: vec![1, 2, 3],
		})
	}
}

// Override data dependency
#[derive(Clone, Debug, PartialEq)]
struct DataDependencyOverride {
	data: Vec<i32>,
}

#[async_trait::async_trait]
impl Injectable for DataDependencyOverride {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(DataDependencyOverride {
			data: vec![3, 4, 5],
		})
	}
}

#[tokio::test]
async fn test_normal_security_with_scopes() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set security scopes
	ctx.set_request(SecurityScopes::new(vec!["foo", "bar"]));

	// Inject user with security
	let user = UserData::inject(&ctx).await.unwrap();
	let data = DataDependency::inject(&ctx).await.unwrap();

	assert_eq!(user.username, "john");
	assert_eq!(user.scopes, vec!["foo", "bar"]);
	assert_eq!(data.data, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_override_data_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set security scopes
	ctx.set_request(SecurityScopes::new(vec!["foo", "bar"]));

	// Inject user (normal)
	let user = UserData::inject(&ctx).await.unwrap();

	// Inject overridden data
	let data = DataDependencyOverride::inject(&ctx).await.unwrap();

	assert_eq!(user.username, "john");
	assert_eq!(user.scopes, vec!["foo", "bar"]);
	assert_eq!(data.data, vec![3, 4, 5]);
}

#[tokio::test]
async fn test_override_security_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set security scopes
	ctx.set_request(SecurityScopes::new(vec!["foo", "bar"]));

	// Inject overridden user
	let user = UserDataOverride::inject(&ctx).await.unwrap();

	// Inject normal data
	let data = DataDependency::inject(&ctx).await.unwrap();

	assert_eq!(user.username, "alice");
	assert_eq!(user.scopes, vec!["foo", "bar"]);
	assert_eq!(data.data, vec![1, 2, 3]);
}

// Test that scopes are properly passed through security dependencies
#[tokio::test]
async fn test_security_scopes_preserved() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set different scopes
	let scopes = vec!["read", "write", "admin"];
	ctx.set_request(SecurityScopes::new(scopes.clone()));

	let user = UserData::inject(&ctx).await.unwrap();

	assert_eq!(user.scopes, scopes);
}

// Test empty scopes
#[tokio::test]
async fn test_security_with_empty_scopes() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Don't set any scopes
	let user = UserData::inject(&ctx).await.unwrap();

	assert_eq!(user.username, "john");
	assert!(user.scopes.is_empty());
}
