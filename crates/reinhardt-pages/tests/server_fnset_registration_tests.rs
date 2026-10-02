#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::server_fn::{
	ServerFnError, ServerFnMetadata, ServerFnRouterExt, ServerFnSet, ServerFnSetChainExt,
	ServerFnSetRegistration, server_fn,
};
use reinhardt_urls::routers::ServerRouter;

#[server_fn]
async fn dashboard() -> Result<String, ServerFnError> {
	Ok(String::from("dashboard"))
}

#[server_fn(codec = "url")]
async fn export(format: String) -> Result<String, ServerFnError> {
	Ok(format)
}

#[server_fn(endpoint = "/api/server_fn/shared")]
async fn first_shared() -> Result<String, ServerFnError> {
	Ok(String::from("first"))
}

#[server_fn(endpoint = "/api/server_fn/shared")]
async fn second_shared() -> Result<String, ServerFnError> {
	Ok(String::from("second"))
}

#[test]
fn named_set_preserves_registration_order_and_member_metadata() {
	// Arrange
	let set = ServerFnSet::new()
		.server_fn(dashboard::marker)
		.server_fn(export::marker)
		.named("admin");

	// Act
	let metadata = set.metadata();
	let _router = ServerRouter::new().server_fnset(set);

	// Assert
	let no_injected_params: &[&str] = &[];
	assert_eq!(metadata.name, "admin");
	assert_eq!(metadata.actions.len(), 2);
	assert_eq!(metadata.actions[0].name, dashboard::marker::NAME);
	assert_eq!(metadata.actions[0].path, dashboard::marker::PATH);
	assert_eq!(metadata.actions[0].codec, "json");
	assert_eq!(metadata.actions[0].injected_params, no_injected_params);
	assert_eq!(metadata.actions[1].name, export::marker::NAME);
	assert_eq!(metadata.actions[1].path, export::marker::PATH);
	assert_eq!(metadata.actions[1].codec, "url");
	assert_eq!(metadata.actions[1].injected_params, no_injected_params);
}

#[test]
fn independently_named_sets_report_duplicate_endpoints_during_compilation() {
	// Arrange
	let first = ServerFnSet::new()
		.server_fn(first_shared::marker)
		.named("first");
	let second = ServerFnSet::new()
		.server_fn(second_shared::marker)
		.named("second");
	let router = ServerRouter::new().server_fnset(first).server_fnset(second);

	// Act
	let errors = router
		.validate_routes()
		.expect_err("duplicate endpoints should fail route compilation");

	// Assert
	assert_eq!(errors.len(), 1);
	assert_eq!(
		errors[0],
		"Failed to compile route '/api/server_fn/shared' (POST): Insertion failed due to conflict with previously registered route: /api/server_fn/shared"
	);
}
