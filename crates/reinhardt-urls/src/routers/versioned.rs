//! [`reinhardt_router::VersionedRouter`] implementations for the
//! concrete router types in this crate.
//!
//! These impls let `reinhardt-rest::versioning` introspect a router's
//! routes through a narrow trait surface, without `reinhardt-rest`
//! having to depend on `reinhardt-urls` directly — which would close
//! the `reinhardt-urls` ↔ `reinhardt-rest` cycle tracked in
//! issue #4321.

use reinhardt_router::{RouteVersionInfo, VersionedRouter};

use super::router::DefaultRouter;
use super::simple::SimpleRouter;

impl VersionedRouter for DefaultRouter {
	fn route_version_infos(&self) -> Vec<RouteVersionInfo> {
		self.get_routes()
			.iter()
			.map(|route| RouteVersionInfo::new(route.namespace.clone(), route.path.clone()))
			.collect()
	}
}

impl VersionedRouter for SimpleRouter {
	fn route_version_infos(&self) -> Vec<RouteVersionInfo> {
		self.get_routes()
			.iter()
			.map(|route| RouteVersionInfo::new(route.namespace.clone(), route.path.clone()))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::routers::{Router, path};
	use async_trait::async_trait;
	use reinhardt_http::{Handler, Request, Response, Result};
	use rstest::rstest;
	use std::sync::Arc;

	struct DummyHandler;

	#[async_trait]
	impl Handler for DummyHandler {
		async fn handle(&self, _req: Request) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	#[rstest]
	fn default_router_empty_yields_no_infos() {
		// Arrange
		let router = DefaultRouter::new();

		// Act
		let infos = router.route_version_infos();

		// Assert
		assert!(infos.is_empty());
	}

	#[rstest]
	fn default_router_surfaces_namespace_and_path() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(DummyHandler);
		let mut router = DefaultRouter::new();
		router.add_route(path("/v1/users/", handler.clone()).with_namespace("v1"));
		router.add_route(path("/v2/users/", handler).with_namespace("v2"));

		// Act
		let infos = router.route_version_infos();

		// Assert
		assert_eq!(infos.len(), 2);
		assert_eq!(infos[0].namespace.as_deref(), Some("v1"));
		assert_eq!(infos[0].path_prefix, "/v1/users/");
		assert_eq!(infos[1].namespace.as_deref(), Some("v2"));
		assert_eq!(infos[1].path_prefix, "/v2/users/");
	}

	#[rstest]
	fn simple_router_surfaces_path() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(DummyHandler);
		let mut router = SimpleRouter::new();
		router.add_route(path("/users/", handler));

		// Act
		let infos = router.route_version_infos();

		// Assert
		assert_eq!(infos.len(), 1);
		assert_eq!(infos[0].path_prefix, "/users/");
	}
}
