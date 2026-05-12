//! Integration tests for the router-aware versioning APIs
//! (`NamespaceVersioning::extract_version_from_router` and
//! `get_available_versions_from_router`).
//!
//! These cover the replacement of the dead-code `_stub` functions
//! removed in issue #4321.

use reinhardt_rest::versioning::NamespaceVersioning;
use reinhardt_router::{RouteVersionInfo, VersionedRouter};
use rstest::{fixture, rstest};

struct FakeRouter {
	infos: Vec<RouteVersionInfo>,
}

impl FakeRouter {
	fn with_versions(pairs: &[(&str, &str)]) -> Self {
		Self {
			infos: pairs
				.iter()
				.map(|(ns, prefix)| {
					RouteVersionInfo::new(Some((*ns).to_string()), (*prefix).to_string())
				})
				.collect(),
		}
	}
}

impl VersionedRouter for FakeRouter {
	fn route_version_infos(&self) -> Vec<RouteVersionInfo> {
		self.infos.clone()
	}
}

#[fixture]
fn v1_and_v2_router() -> FakeRouter {
	FakeRouter::with_versions(&[("v1", "/v1/users/"), ("v2", "/v2/users/")])
}

#[rstest]
fn extract_version_from_router_returns_pattern_match(v1_and_v2_router: FakeRouter) {
	// Arrange
	let versioning = NamespaceVersioning::new().with_pattern("/v{version}/");

	// Act
	let version = versioning.extract_version_from_router(&v1_and_v2_router, "/v2/users/");

	// Assert
	assert_eq!(version, Some("2".to_string()));
}

#[rstest]
fn extract_version_from_router_returns_none_for_non_matching_path(v1_and_v2_router: FakeRouter) {
	// Arrange
	let versioning = NamespaceVersioning::new().with_pattern("/v{version}/");

	// Act
	let version = versioning.extract_version_from_router(&v1_and_v2_router, "/health");

	// Assert
	assert_eq!(version, None);
}

#[rstest]
fn get_available_versions_from_router_enumerates_registered(v1_and_v2_router: FakeRouter) {
	// Arrange
	let versioning = NamespaceVersioning::new().with_pattern("/v{version}/");

	// Act
	let versions = versioning.get_available_versions_from_router(&v1_and_v2_router);

	// Assert
	assert_eq!(versions, vec!["1".to_string(), "2".to_string()]);
}

#[rstest]
fn get_available_versions_from_router_filters_by_allowed_versions() {
	// Arrange
	let router = FakeRouter::with_versions(&[
		("v1", "/v1/users/"),
		("v2", "/v2/users/"),
		("v3", "/v3/users/"),
	]);
	let versioning = NamespaceVersioning::new()
		.with_pattern("/v{version}/")
		.with_allowed_versions(vec!["1", "2"]);

	// Act
	let versions = versioning.get_available_versions_from_router(&router);

	// Assert
	assert_eq!(versions, vec!["1".to_string(), "2".to_string()]);
}

#[rstest]
fn get_available_versions_from_router_returns_empty_for_empty_router() {
	// Arrange
	let router = FakeRouter::with_versions(&[]);
	let versioning = NamespaceVersioning::new().with_pattern("/v{version}/");

	// Act
	let versions = versioning.get_available_versions_from_router(&router);

	// Assert
	assert!(versions.is_empty());
}

#[rstest]
fn get_available_versions_from_router_deduplicates() {
	// Arrange
	let router = FakeRouter::with_versions(&[
		("v1", "/v1/users/"),
		("v1", "/v1/posts/"),
		("v2", "/v2/users/"),
	]);
	let versioning = NamespaceVersioning::new().with_pattern("/v{version}/");

	// Act
	let versions = versioning.get_available_versions_from_router(&router);

	// Assert
	assert_eq!(versions, vec!["1".to_string(), "2".to_string()]);
}
