use crate::evaluation::{TraceGuard, traced_urls};
use reinhardt::ClientRouter;
use reinhardt::reinhardt_urls::routers::client_router::RouterError;
use reinhardt::reinhardt_urls::routers::registration::iter_registered_client_routers;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn assert_client_routes(client: &ClientRouter) {
	assert_eq!(client.route_count(), 2);
	assert_eq!(client.reverse("home", &[]).unwrap(), "/");
	assert_eq!(client.reverse("auth:login", &[]).unwrap(), "/login/");
	for native_name in ["native-page-health", "demo:health", "demo:protected"] {
		assert_eq!(
			client.reverse(native_name, &[]),
			Err(RouterError::InvalidRouteName(String::from(native_name)))
		);
	}
}

#[wasm_bindgen_test]
fn client_routes_survive_server_erasure_and_mounting() {
	// Arrange
	let trace = TraceGuard::new();

	// Act
	let client = crate::client_pages().into_client();

	// Assert
	assert_client_routes(&client);
	assert_eq!(trace.take(), vec!["client-argument"]);
}

#[wasm_bindgen_test]
fn unrelated_server_method_is_preserved() {
	// Arrange / Act
	let client = crate::unrelated_server_call().into_client();

	// Assert
	assert_eq!(client.route_count(), 1);
	assert_eq!(client.reverse("extra", &[]).unwrap(), "/extra/");
}

#[wasm_bindgen_test]
fn native_arguments_captures_and_bodies_are_not_evaluated() {
	// Arrange
	let trace = TraceGuard::new();

	// Act
	let client = traced_urls().into_client();

	// Assert
	assert_eq!(client.route_count(), 0);
	assert_eq!(
		trace.take(),
		vec!["prefix", "namespace", "merge-argument", "drop-borrow"]
	);
}

#[wasm_bindgen_test]
fn only_the_root_attribute_registers_the_client_factory() {
	// Arrange
	let expected = usize::from(cfg!(any(
		feature = "routes-first",
		feature = "url-patterns-first"
	)));

	// Act
	let registrations: Vec<_> = iter_registered_client_routers().collect();

	// Assert
	assert_eq!(registrations.len(), expected);
	for registration in registrations {
		assert_client_routes(&registration.client_router());
	}
}
