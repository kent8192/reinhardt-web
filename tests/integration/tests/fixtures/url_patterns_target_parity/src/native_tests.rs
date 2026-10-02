use crate::evaluation::{TraceGuard, traced_urls};
use rstest::{fixture, rstest};
use serial_test::serial;

#[fixture]
fn trace() -> TraceGuard {
	TraceGuard::new()
}

#[rstest]
#[serial(url_patterns_evaluation)]
fn evaluation_and_temporary_lifetimes_are_preserved(trace: TraceGuard) {
	// Arrange
	#[cfg(server)]
	let expected = vec![
		"prefix",
		"server-argument-1",
		"server-body-1",
		"drop-capture",
		"namespace",
		"merge-argument",
		"server-argument-2",
		"server-body-2",
		"drop-borrow",
	];
	#[cfg(not(server))]
	let expected = vec!["prefix", "namespace", "merge-argument", "drop-borrow"];

	// Act
	let generated = traced_urls().into_server();

	// Assert
	assert_eq!(generated.namespace(), Some("trace"));
	assert_eq!(generated.prefix(), "/api/");
	assert_eq!(trace.take(), expected);
	#[cfg(server)]
	{
		let handwritten = crate::evaluation::handwritten_traced_urls().into_server();
		assert_eq!(trace.take(), expected);
		assert_eq!(generated.get_all_routes(), handwritten.get_all_routes());
	}
}

#[cfg(not(server))]
#[rstest]
fn non_server_native_build_has_no_contributed_endpoints() {
	// Arrange / Act
	let router = crate::url_patterns().into_server();

	// Assert
	assert_eq!(router.get_all_routes(), Vec::new());
	assert_eq!(router.reverse("demo:health", &[]), None);
	assert_eq!(router.reverse("demo:protected", &[]), None);
}

#[cfg(feature = "client-router")]
#[rstest]
#[serial(url_patterns_evaluation)]
fn client_configuration_arguments_are_evaluated_once(trace: TraceGuard) {
	// Arrange / Act
	let router = crate::client_pages().into_server();

	// Assert
	assert_eq!(trace.take(), vec!["client-argument"]);
	assert_eq!(router.get_all_routes().len(), usize::from(cfg!(server)));
}

#[cfg(server)]
mod server {
	use super::trace;
	use crate::evaluation::TraceGuard;
	use reinhardt::reinhardt_core::endpoint::AuthProtection;
	use reinhardt::urls::prelude::UnifiedRouter;
	use reinhardt::{Handler, Method, Request, StatusCode};
	use rstest::rstest;
	use serial_test::serial;

	fn handwritten() -> UnifiedRouter {
		UnifiedRouter::new()
			.server(|server| {
				server
					.endpoint(crate::native_handlers::health)
					.endpoint(crate::native_handlers::protected)
			})
			.with_namespace("demo")
	}

	#[reinhardt::url_patterns]
	fn guarded() -> UnifiedRouter {
		UnifiedRouter::new()
			.server(crate::native_support::configure)
			.with_prefix("/api/")
	}

	fn handwritten_guarded() -> UnifiedRouter {
		UnifiedRouter::new()
			.server(crate::native_support::configure)
			.with_prefix("/api/")
	}

	#[reinhardt::url_patterns]
	fn prefix_before() -> UnifiedRouter {
		UnifiedRouter::new()
			.with_prefix("/api/")
			.server(|server| server.endpoint(crate::native_handlers::health))
	}

	#[reinhardt::url_patterns]
	fn prefix_after() -> UnifiedRouter {
		UnifiedRouter::new()
			.server(|server| server.endpoint(crate::native_handlers::health))
			.with_prefix("/api/")
	}

	#[reinhardt::url_patterns]
	fn nested() -> UnifiedRouter {
		UnifiedRouter::new()
			.with_prefix("/api/")
			.with_namespace("root")
			.mount_unified("/app/", crate::url_patterns())
			.merge(crate::another_app())
	}

	#[reinhardt::url_patterns]
	fn duplicate() -> UnifiedRouter {
		UnifiedRouter::new().server(|server| {
			server
				.endpoint(crate::native_handlers::health)
				.endpoint(crate::native_handlers::health)
		})
	}

	#[rstest]
	fn endpoints_and_contract_metadata_match_the_handwritten_builder() {
		// Arrange
		let expected = vec![
			(
				String::from("/health/"),
				Some(String::from("health")),
				Some(String::from("demo")),
				vec![Method::GET],
			),
			(
				String::from("/protected/"),
				Some(String::from("protected")),
				Some(String::from("demo")),
				vec![Method::POST],
			),
		];

		// Act
		let generated = crate::url_patterns().into_server();
		let original = handwritten().into_server();
		let contracts = generated.get_mounted_route_contracts().unwrap();

		// Assert
		assert_eq!(generated.get_all_routes(), expected);
		assert_eq!(generated.get_all_routes(), original.get_all_routes());
		assert_eq!(contracts, original.get_mounted_route_contracts().unwrap());
		assert_eq!(contracts.len(), 2);
		assert_eq!(contracts[0].metadata.authentication, AuthProtection::Public);
		assert_eq!(
			contracts[1].metadata.authentication,
			AuthProtection::Protected
		);
		assert_eq!(
			contracts[1].metadata.guard.as_deref(),
			Some("fixture credential gate")
		);
		assert_eq!(
			generated.reverse("demo:health", &[]).as_deref(),
			Some("/health/")
		);
		assert_eq!(
			generated.reverse("demo:protected", &[]).as_deref(),
			Some("/protected/")
		);
	}

	#[rstest]
	#[case(Method::GET, "/health/", StatusCode::OK, "native-handler")]
	#[case(Method::POST, "/protected/", StatusCode::OK, "protected-handler")]
	#[tokio::test]
	async fn actual_http_handlers_are_dispatched(
		#[case] method: Method,
		#[case] path: &str,
		#[case] status: StatusCode,
		#[case] body: &str,
	) {
		// Arrange
		let router = crate::url_patterns().into_server();
		let request = Request::builder().method(method).uri(path).build().unwrap();

		// Act
		let response = router.handle(request).await.unwrap();

		// Assert
		assert_eq!(response.status, status);
		assert_eq!(response.body.as_ref(), body.as_bytes());
	}

	#[rstest]
	#[case(
		Method::POST,
		"/health/",
		405,
		"Method not allowed: Method POST not allowed for /health/"
	)]
	#[case(Method::GET, "/missing/", 404, "Not found: No route for GET /missing/")]
	#[tokio::test]
	async fn failed_dispatch_matches_the_original_router(
		#[case] method: Method,
		#[case] path: &str,
		#[case] status: u16,
		#[case] message: &str,
	) {
		// Arrange
		let request = || {
			Request::builder()
				.method(method.clone())
				.uri(path)
				.build()
				.unwrap()
		};
		let generated = crate::url_patterns().into_server();
		let original = handwritten().into_server();

		// Act
		let actual = generated.handle(request()).await.unwrap_err();
		let expected = original.handle(request()).await.unwrap_err();

		// Assert
		assert_eq!(actual.status_code(), status);
		assert_eq!(actual.to_string(), message);
		assert_eq!(actual.kind(), expected.kind());
		assert_eq!(actual.to_string(), expected.to_string());
	}

	#[rstest]
	#[case(false, StatusCode::FORBIDDEN, "denied", vec!["denied"])]
	#[case(true, StatusCode::OK, "injected-configuration", vec!["middleware-before", "handler", "middleware-after"])]
	#[serial(url_patterns_evaluation)]
	#[tokio::test]
	async fn middleware_enforcement_and_injection_are_preserved(
		trace: TraceGuard,
		#[case] accepted: bool,
		#[case] status: StatusCode,
		#[case] body: &str,
		#[case] events: Vec<&str>,
	) {
		// Arrange
		let request = || {
			Request::builder()
				.method(Method::GET)
				.uri("/api/secured/")
				.header(
					"x-fixture-credential",
					if accepted { "accepted" } else { "rejected" },
				)
				.build()
				.unwrap()
		};

		// Act
		let generated = guarded().into_server();
		let response = generated.handle(request()).await.unwrap();

		// Assert
		assert_eq!(response.status, status);
		assert_eq!(response.body.as_ref(), body.as_bytes());
		assert_eq!(trace.take(), events);
		let original = handwritten_guarded().into_server();
		let expected = original.handle(request()).await.unwrap();
		assert_eq!(trace.take(), events);
		assert_eq!(response.status, expected.status);
		assert_eq!(response.body, expected.body);
		assert_eq!(
			generated.get_registered_middleware(),
			original.get_registered_middleware()
		);
		assert_eq!(
			generated.get_mounted_route_contracts().unwrap(),
			original.get_mounted_route_contracts().unwrap()
		);
	}

	#[rstest]
	fn prefixes_mounts_namespaces_and_merge_keep_literal_reversal_paths() {
		// Arrange / Act
		let before = prefix_before().into_server();
		let after = prefix_after().into_server();
		let parent = nested().into_server();

		// Assert
		assert_eq!(
			before.reverse("health", &[]).as_deref(),
			Some("/api/health/")
		);
		assert_eq!(
			after.reverse("health", &[]).as_deref(),
			Some("/api/health/")
		);
		assert_eq!(before.get_all_routes(), after.get_all_routes());
		assert_eq!(
			parent.reverse("root:demo:health", &[]).as_deref(),
			Some("/api/app/health/")
		);
		assert_eq!(
			parent.reverse("root:demo:protected", &[]).as_deref(),
			Some("/api/app/protected/")
		);
		assert_eq!(
			parent.reverse("root:health", &[]).as_deref(),
			Some("/api/other/health/")
		);
		assert_eq!(parent.get_all_routes().len(), 3);
	}

	#[rstest]
	fn duplicate_endpoint_contracts_keep_the_original_error() {
		// Arrange
		let original = UnifiedRouter::new()
			.server(|server| {
				server
					.endpoint(crate::native_handlers::health)
					.endpoint(crate::native_handlers::health)
			})
			.into_server();

		// Act
		let actual = duplicate().into_server().get_mounted_route_contracts();

		// Assert
		assert_eq!(actual, original.get_mounted_route_contracts());
		assert_eq!(
			actual.unwrap_err(),
			"route compilation failed: Failed to compile route '/health/' (GET): Insertion failed due to conflict with previously registered route: /health/"
		);
	}
}
