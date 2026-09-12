//! Consumer declarations with real native-only endpoint references.

#![deny(warnings)]

use reinhardt::url_patterns;
use reinhardt::urls::prelude::UnifiedRouter;

#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
mod native_handlers;

#[cfg(test)]
mod evaluation;

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
mod native_tests;

#[cfg(all(test, server, not(all(target_family = "wasm", target_os = "unknown"))))]
mod native_support;

#[cfg(feature = "client-router")]
mod client_routes;

#[cfg(feature = "client-router")]
pub use client_routes::{client_pages, unrelated_server_call};

#[cfg(all(
	test,
	feature = "client-router",
	target_family = "wasm",
	target_os = "unknown"
))]
mod wasm_tests;

#[url_patterns]
pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(|server| {
			server
				.endpoint(crate::native_handlers::health)
				.endpoint(crate::native_handlers::protected)
		})
		.with_namespace("demo")
}

#[reinhardt::url_patterns]
pub fn another_app() -> reinhardt::urls::prelude::UnifiedRouter {
	reinhardt::urls::prelude::UnifiedRouter::default()
		.server(|server| server.endpoint(crate::native_handlers::health))
		.with_prefix("/other/")
}

#[cfg(feature = "routes-first")]
mod registration_routes_first;

#[cfg(feature = "url-patterns-first")]
mod registration_url_patterns_first;

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
mod registration_tests {
	use reinhardt::reinhardt_urls::routers::registration::iter_registered_url_patterns;
	use rstest::rstest;

	#[rstest]
	fn only_the_root_attribute_registers_inventory() {
		// Arrange
		let expected = usize::from(cfg!(any(
			feature = "routes-first",
			feature = "url-patterns-first"
		)));

		// Act
		let registrations: Vec<_> = iter_registered_url_patterns().collect();

		// Assert
		assert_eq!(registrations.len(), expected);
		for registration in registrations {
			let router = registration.server_router();
			#[cfg(server)]
			{
				assert_eq!(router.get_all_routes().len(), 5);
				assert_eq!(
					router.reverse("demo:health", &[]).as_deref(),
					Some("/app/health/")
				);
				assert_eq!(
					router.reverse("native-page-health", &[]).as_deref(),
					Some("/native-pages/health/")
				);
			}
			#[cfg(not(server))]
			assert_eq!(router.get_all_routes(), Vec::new());
		}
	}
}
