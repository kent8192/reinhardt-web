use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use reinhardt_http::{Handler, Request, Response};
use reinhardt_test::APIClient;
use rstest::*;
use serde::Serialize;

struct EchoHandler;

#[async_trait]
impl Handler for EchoHandler {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		let path = request.uri.path().to_string();
		let method = request.method.as_str().to_string();
		let custom_header = request
			.headers
			.get("X-Custom")
			.and_then(|value| value.to_str().ok());
		let content_type = request
			.headers
			.get("Content-Type")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("")
			.to_string();
		let request_header = request
			.headers
			.get("X-Request")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let raw_header = request
			.headers
			.get("X-Raw")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let body = String::from_utf8_lossy(request.body()).into_owned();

		let mut response = Response::ok().with_body(path.clone());
		response = response.try_with_header("X-Echo-Path", &path)?;
		response = response.try_with_header("X-Echo-Method", &method)?;
		response = response.try_with_header("X-Echo-X-Request", request_header)?;
		response = response.try_with_header("X-Echo-X-Raw", raw_header)?;
		response = response.try_with_header("X-Echo-Body", &body)?;

		if let Some(custom_header) = custom_header {
			response = response.try_with_header("X-Echo-Custom", custom_header)?;
		}
		if !content_type.is_empty() {
			response = response.try_with_header("X-Echo-Content-Type", &content_type)?;
		}
		Ok(response)
	}
}

struct RequestMetadataHandler;

#[async_trait]
impl Handler for RequestMetadataHandler {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		let request_uri = request.uri.to_string();
		let authorization = request
			.headers
			.get(http::header::AUTHORIZATION)
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let cookie = request
			.headers
			.get(http::header::COOKIE)
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let custom = request
			.headers
			.get("X-Custom")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let mfa_code = request
			.headers
			.get("X-MFA-Code")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");
		let test_user = request
			.headers
			.get("X-Test-User")
			.and_then(|value| value.to_str().ok())
			.unwrap_or("missing");

		Response::ok()
			.try_with_header("X-Request-Uri", &request_uri)?
			.try_with_header("X-Authorization", authorization)?
			.try_with_header("X-Cookie", cookie)?
			.try_with_header("X-MFA-Code", mfa_code)?
			.try_with_header("X-Test-User", test_user)?
			.try_with_header("X-Custom", custom)
	}
}

#[rstest]
#[tokio::test]
async fn api_client_dispatches_all_public_http_methods_and_payload_formats() {
	#[derive(Serialize)]
	struct Payload {
		name: &'static str,
		count: u8,
	}

	// Arrange
	let client = APIClient::from_handler(EchoHandler);
	let payload = Payload {
		name: "Ada",
		count: 2,
	};
	client.set_header("X-Custom", "client-value").await.unwrap();
	client.set_cookie("sessionid", "session-42").await.unwrap();

	// Act
	let request: reinhardt::TestRequestBuilder<'_> = client.request(http::Method::GET, "/get");
	let get = request.header("X-Request", "get").send().await.unwrap();
	let post = client.post("/post", &payload, "json").await.unwrap();
	let put = client.put("/put", &payload, "form").await.unwrap();
	let patch = client.patch("/patch", &payload, "json").await.unwrap();
	let delete = client.delete("/delete").await.unwrap();
	let head = client.head("/head").await.unwrap();
	let options = client.options("/options").await.unwrap();
	let raw_headers = client
		.request(http::Method::POST, "/raw-headers")
		.body(bytes::Bytes::copy_from_slice(b"raw-body"))
		.header(http::header::CONTENT_TYPE, "text/plain")
		.header("X-Raw", "yes")
		.send()
		.await
		.unwrap();
	let raw = client
		.post_raw("/raw", b"bytes", "application/octet-stream")
		.await
		.unwrap();
	let form_error = match client
		.post("/invalid-form", &vec!["not-an-object"], "form")
		.await
	{
		Err(error) => error,
		Ok(_) => panic!("expected form serialization to reject an array"),
	};
	let format_error = match client.post("/invalid-format", &payload, "yaml").await {
		Err(error) => error,
		Ok(_) => panic!("expected unsupported format to fail"),
	};

	// Assert
	for (response, path, method) in [
		(&get, "/get", "GET"),
		(&post, "/post", "POST"),
		(&put, "/put", "PUT"),
		(&patch, "/patch", "PATCH"),
		(&delete, "/delete", "DELETE"),
		(&head, "/head", "HEAD"),
		(&options, "/options", "OPTIONS"),
		(&raw_headers, "/raw-headers", "POST"),
		(&raw, "/raw", "POST"),
	] {
		assert_eq!(response.status(), http::StatusCode::OK);
		assert_eq!(response.body().as_ref(), path.as_bytes());
		assert_eq!(response.header("X-Echo-Method"), Some(method));
		assert_eq!(response.header("X-Echo-Custom"), Some("client-value"));
	}
	assert_eq!(post.header("X-Echo-Content-Type"), Some("application/json"));
	assert_eq!(get.header("X-Echo-X-Request"), Some("get"));
	assert_eq!(raw_headers.header("X-Echo-X-Raw"), Some("yes"));
	assert_eq!(
		post.header("X-Echo-Body"),
		Some(r#"{"name":"Ada","count":2}"#)
	);
	assert_eq!(put.header("X-Echo-Body"), Some("name=Ada&count=2"));
	assert_eq!(
		patch.header("X-Echo-Body"),
		Some(r#"{"name":"Ada","count":2}"#)
	);
	assert_eq!(raw_headers.header("X-Echo-Body"), Some("raw-body"));
	assert_eq!(raw.header("X-Echo-Body"), Some("bytes"));
	assert_eq!(
		put.header("X-Echo-Content-Type"),
		Some("application/x-www-form-urlencoded")
	);
	assert_eq!(
		patch.header("X-Echo-Content-Type"),
		Some("application/json")
	);
	assert_eq!(
		raw_headers.header("X-Echo-Content-Type"),
		Some("text/plain")
	);
	assert_eq!(
		raw.header("X-Echo-Content-Type"),
		Some("application/octet-stream")
	);
	assert_eq!(
		form_error.to_string(),
		"Request failed: Expected object for form data"
	);
	assert_eq!(
		format_error.to_string(),
		"Request failed: Unsupported format: yaml"
	);
	assert!(form_error.is_request());
	assert!(!format_error.is_timeout());
	assert!(!format_error.is_connect());
}

#[rstest]
#[tokio::test]
async fn api_client_manages_credentials_cookies_and_cleanup_through_requests() {
	// Arrange
	let client = APIClient::from_handler(RequestMetadataHandler);
	let first_password = std::process::id().to_string();
	let logout_password = format!("{}-logout", std::process::id());
	client.set_header("X-Custom", "retained").await.unwrap();
	client.credentials("ada", &first_password).await.unwrap();
	client.set_cookie("session", "first").await.unwrap();

	// Act
	let authenticated = client.get("/authenticated").await.unwrap();
	client.remove_cookie("session").await.unwrap();
	let without_cookie = client.get("/without-cookie").await.unwrap();
	client.set_cookie("session", "second").await.unwrap();
	client
		.set_header("X-MFA-Code", "before-clear")
		.await
		.unwrap();
	client
		.set_header("X-Test-User", "authenticated")
		.await
		.unwrap();
	client.clear_auth().await.unwrap();
	let cleared_auth = client.get("/cleared-auth").await.unwrap();
	client.set_header("X-MFA-Code", "123456").await.unwrap();
	client
		.set_header("X-Test-User", "authenticated")
		.await
		.unwrap();
	client.set_cookie("session", "third").await.unwrap();
	client.cleanup().await;
	let cleaned = client.get("https://example.test/cleaned").await.unwrap();
	client
		.set_header("X-Custom", "logout-retained")
		.await
		.unwrap();
	client.credentials("grace", &logout_password).await.unwrap();
	client
		.set_header("X-MFA-Code", "before-logout")
		.await
		.unwrap();
	client
		.set_header("X-Test-User", "authenticated")
		.await
		.unwrap();
	client
		.set_cookie("session", "logout-session")
		.await
		.unwrap();
	client.logout().await.unwrap();
	let logged_out = client.get("/logged-out").await.unwrap();

	// Assert
	let encoded_authorization = authenticated
		.header("X-Authorization")
		.unwrap()
		.strip_prefix("Basic ")
		.unwrap();
	let decoded_authorization = general_purpose::STANDARD
		.decode(encoded_authorization)
		.unwrap();
	assert_eq!(
		String::from_utf8(decoded_authorization).unwrap(),
		format!("ada:{}", first_password)
	);
	assert_eq!(authenticated.header("X-Cookie"), Some("session=first"));
	assert_eq!(authenticated.header("X-Custom"), Some("retained"));
	assert_eq!(without_cookie.header("X-Cookie"), Some("missing"));
	assert_eq!(cleared_auth.header("X-Authorization"), Some("missing"));
	assert_eq!(cleared_auth.header("X-Cookie"), Some("missing"));
	assert_eq!(cleared_auth.header("X-MFA-Code"), Some("missing"));
	assert_eq!(cleared_auth.header("X-Test-User"), Some("missing"));
	assert_eq!(cleared_auth.header("X-Custom"), Some("retained"));
	assert_eq!(
		cleaned.header("X-Request-Uri"),
		Some("https://example.test/cleaned")
	);
	assert_eq!(cleaned.header("X-Authorization"), Some("missing"));
	assert_eq!(cleaned.header("X-Cookie"), Some("missing"));
	assert_eq!(cleaned.header("X-MFA-Code"), Some("missing"));
	assert_eq!(cleaned.header("X-Test-User"), Some("missing"));
	assert_eq!(cleaned.header("X-Custom"), Some("missing"));
	assert_eq!(logged_out.header("X-Authorization"), Some("missing"));
	assert_eq!(logged_out.header("X-Cookie"), Some("missing"));
	assert_eq!(logged_out.header("X-MFA-Code"), Some("missing"));
	assert_eq!(logged_out.header("X-Test-User"), Some("missing"));
	assert_eq!(logged_out.header("X-Custom"), Some("logout-retained"));
}

#[rstest]
#[tokio::test]
async fn fork_uses_the_same_handler_and_di_context() {
	use reinhardt_di::{InjectionContext, SingletonScope};
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	struct ContextHandler {
		context: Arc<InjectionContext>,
		calls: Arc<AtomicUsize>,
	}
	#[async_trait]
	impl Handler for ContextHandler {
		async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
			let context = request.get_di_context::<Arc<InjectionContext>>().unwrap();
			assert!(Arc::ptr_eq(context.as_ref(), &self.context));
			let call = self.calls.fetch_add(1, Ordering::SeqCst);
			Ok(Response::ok().with_body(format!("call-{call}")))
		}
	}
	// Arrange
	let context = Arc::new(InjectionContext::builder(SingletonScope::new()).build());
	let calls = Arc::new(AtomicUsize::new(0));
	let parent = APIClient::builder()
		.handler(ContextHandler {
			context: Arc::clone(&context),
			calls: Arc::clone(&calls),
		})
		.di_context(context)
		.build();
	let fork = parent.fork().await;
	// Act
	let parent_response = parent.get("/parent").await.unwrap();
	let fork_response = fork.get("/fork").await.unwrap();
	// Assert
	assert_eq!(parent_response.text(), "call-0");
	assert_eq!(fork_response.text(), "call-1");
	assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[rstest]
#[tokio::test]
async fn fork_has_an_independent_automatic_cookie_jar_and_inherits_manual_cookies() {
	use reinhardt_test::{fixtures::test_server_guard, stub::StubRouter};
	// Arrange
	let router = StubRouter::new()
		.get("/set-parent", |_| async {
			Ok(Response::ok().with_header("set-cookie", "automatic=parent; Path=/"))
		})
		.get("/set-fork", |_| async {
			Ok(Response::ok().with_header("set-cookie", "automatic=fork; Path=/"))
		})
		.get("/cookies", |request| async move {
			let cookie = request
				.headers
				.get(http::header::COOKIE)
				.and_then(|value| value.to_str().ok())
				.unwrap_or("missing");
			Ok(Response::ok().with_body(cookie.to_owned()))
		});
	let server = test_server_guard(router.into()).await;
	let parent = APIClient::builder()
		.base_url(&server.url)
		.cookie_store(true)
		.http1_only()
		.build();
	parent.get("/set-parent").await.unwrap();
	let fork = parent.fork().await;
	// Act
	let parent_cookie = parent.get("/cookies").await.unwrap();
	let fork_before_cookie = fork.get("/cookies").await.unwrap();
	fork.get("/set-fork").await.unwrap();
	let fork_cookie = fork.get("/cookies").await.unwrap();
	let parent_after_cookie = parent.get("/cookies").await.unwrap();
	parent.set_cookie("manual", "snapshot").await.unwrap();
	let manual_fork = parent.fork().await;
	let manual_cookie = manual_fork.get("/cookies").await.unwrap();
	let suppressed_manual = manual_fork
		.request(http::Method::GET, "/cookies")
		.without_header(http::header::COOKIE)
		.send()
		.await
		.unwrap();
	let explicit = parent
		.request(http::Method::GET, "/cookies")
		.header(http::header::COOKIE, "explicit=request")
		.send()
		.await
		.unwrap();
	let automatic_after_removal = fork
		.request(http::Method::GET, "/cookies")
		.without_header(http::header::COOKIE)
		.send()
		.await
		.unwrap();
	// Assert
	assert_eq!(parent_cookie.text(), "automatic=parent");
	assert_eq!(fork_before_cookie.text(), "missing");
	assert_eq!(fork_cookie.text(), "automatic=fork");
	assert_eq!(parent_after_cookie.text(), "automatic=parent");
	assert_eq!(manual_cookie.text(), "manual=snapshot");
	assert_eq!(suppressed_manual.text(), "missing");
	assert_eq!(explicit.text(), "explicit=request");
	assert_eq!(automatic_after_removal.text(), "automatic=fork");
	assert_eq!(fork_cookie.version(), http::Version::HTTP_11);
}

#[rstest]
#[tokio::test]
async fn fork_inherits_transport_timeout() {
	use reinhardt_test::{fixtures::test_server_guard, stub::StubRouter};
	use std::time::Duration;
	// Arrange
	let router = StubRouter::new().get("/slow", |_| async {
		tokio::time::sleep(Duration::from_secs(1)).await;
		Ok(Response::ok())
	});
	let server = test_server_guard(router.into()).await;
	let parent = APIClient::builder()
		.base_url(&server.url)
		.timeout(Duration::from_millis(50))
		.build();
	let fork = parent.fork().await;
	// Act
	let result = fork.get("/slow").await;
	// Assert
	let error = match result {
		Err(error) => error,
		Ok(_) => panic!("expected inherited timeout"),
	};
	assert!(error.is_timeout());
}
