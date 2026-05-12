//! The [`VersionedRouter`] trait.

use crate::version::RouteVersionInfo;

/// A router that can expose its routes as [`RouteVersionInfo`] entries.
///
/// `reinhardt-urls`' concrete router types implement this trait; the
/// generic `reinhardt-rest::versioning` API consumes it without ever
/// importing `reinhardt-urls` directly. That is what breaks the
/// `reinhardt-urls` ↔ `reinhardt-rest` circular dependency tracked in
/// issue #4321.
///
/// # Implementation notes
///
/// - Implementations SHOULD return one entry per registered route, in
///   registration order. Deduplication is the caller's responsibility.
/// - The trait is intentionally narrow: just enough to drive
///   namespace- and path-based versioning strategies. Concrete router
///   capabilities (resolving, dispatching, middleware) stay on the
///   concrete type and out of this crate.
pub trait VersionedRouter {
	/// Returns one [`RouteVersionInfo`] entry per registered route.
	fn route_version_infos(&self) -> Vec<RouteVersionInfo>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct MockRouter(Vec<RouteVersionInfo>);

	impl VersionedRouter for MockRouter {
		fn route_version_infos(&self) -> Vec<RouteVersionInfo> {
			self.0.clone()
		}
	}

	#[rstest]
	fn returns_registered_infos_in_order() {
		// Arrange
		let router = MockRouter(vec![
			RouteVersionInfo::new(Some("v1".into()), "/v1/users/"),
			RouteVersionInfo::new(Some("v2".into()), "/v2/users/"),
		]);

		// Act
		let infos = router.route_version_infos();

		// Assert
		assert_eq!(infos.len(), 2);
		assert_eq!(infos[0].namespace.as_deref(), Some("v1"));
		assert_eq!(infos[1].path_prefix, "/v2/users/");
	}

	#[rstest]
	fn empty_router_yields_empty_vec() {
		// Arrange
		let router = MockRouter(vec![]);

		// Act
		let infos = router.route_version_infos();

		// Assert
		assert!(infos.is_empty());
	}
}
