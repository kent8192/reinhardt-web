//! Tests for feature flags in reinhardt-auth
//!
//! These tests verify conditional compilation based on feature flags,
//! particularly the `params` feature for auth extractors.
//!
//! `AuthInfo` is exercised against an authenticated request with `params` enabled
//! and an empty injection context with `params` disabled.

// ============================================================================
// Tests with params feature disabled
// ============================================================================

#[cfg(not(feature = "params"))]
mod params_feature_disabled {
	use reinhardt_di::{DiError, Injectable, InjectionContext, SingletonScope};
	use rstest::rstest;

	fn create_empty_context() -> InjectionContext {
		let singleton_scope = SingletonScope::new();
		InjectionContext::builder(singleton_scope).build()
	}

	#[rstest]
	#[tokio::test]
	async fn test_auth_info_returns_not_found_without_params() {
		// Arrange
		let ctx = create_empty_context();

		// Act
		let result = reinhardt_auth::AuthInfo::inject(&ctx).await;

		// Assert
		assert!(
			result.is_err(),
			"AuthInfo should fail without params feature"
		);
		match result.unwrap_err() {
			DiError::NotFound(msg) => {
				assert_eq!(msg, "AuthInfo requires the 'params' feature to be enabled");
			}
			other => panic!("Expected DiError::NotFound, got: {other:?}"),
		}
	}
}

// ============================================================================
// Tests with params feature enabled
// ============================================================================

#[cfg(feature = "params")]
mod params_feature_enabled {
	use reinhardt_auth::AuthInfo;
	use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
	use reinhardt_http::{AuthState, Request};
	use rstest::rstest;

	#[rstest]
	#[case("alice", false, true)]
	#[case("admin", true, false)]
	#[tokio::test]
	async fn test_auth_info_reads_authenticated_request_with_params(
		#[case] user_id: &str,
		#[case] is_admin: bool,
		#[case] is_active: bool,
	) {
		// Arrange
		let request = Request::builder().uri("/").build().unwrap();
		request
			.extensions
			.insert(AuthState::authenticated(user_id, is_admin, is_active));
		let ctx = InjectionContext::builder(SingletonScope::new())
			.with_request(request)
			.build();

		// Act
		let AuthInfo(state) = AuthInfo::inject(&ctx)
			.await
			.expect("AuthInfo should read the authenticated request with params enabled");

		// Assert
		assert_eq!(state.user_id(), user_id);
		assert!(state.is_authenticated());
		assert_eq!(state.is_admin(), is_admin);
		assert_eq!(state.is_active(), is_active);
	}
}
