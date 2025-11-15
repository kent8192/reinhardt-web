//! Integration tests for Server implementation

use hyper::Method;
use reinhardt_http::Response;
use reinhardt_routers::UnifiedRouter as Router;
use reinhardt_test::fixtures::*;
use std::sync::Arc;

#[tokio::test]
async fn test_server_basic_request() {
	let router = Arc::new(Router::new().function("/test", Method::GET, |_req| async {
		Ok(Response::ok().with_body("Server works!"))
	}));

	let server = test_server_guard(router).await;

	// Make HTTP request
	let client = reqwest::Client::new();
	let response = client
		.get(format!("{}/test", server.url))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::OK);
	assert_eq!(response.text().await.unwrap(), "Server works!");
}

#[tokio::test]
async fn test_server_multiple_requests() {
	let router = Arc::new(
		Router::new()
			.function("/hello", Method::GET, |_req| async {
				Ok(Response::ok().with_body("Hello!"))
			})
			.function("/goodbye", Method::GET, |_req| async {
				Ok(Response::ok().with_body("Goodbye!"))
			}),
	);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();

	let response = client
		.get(format!("{}/hello", server.url))
		.send()
		.await
		.unwrap();
	assert_eq!(response.text().await.unwrap(), "Hello!");

	let response = client
		.get(format!("{}/goodbye", server.url))
		.send()
		.await
		.unwrap();
	assert_eq!(response.text().await.unwrap(), "Goodbye!");
}

#[tokio::test]
async fn test_server_post_request() {
	let router = Arc::new(
		Router::new().function("/submit", Method::POST, |request| async move {
			let body_str = String::from_utf8(request.body().to_vec()).unwrap_or_default();
			Ok(Response::ok().with_body(format!("Received: {}", body_str)))
		}),
	);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/submit", server.url))
		.body("test data")
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::OK);
	assert_eq!(response.text().await.unwrap(), "Received: test data");
}

#[tokio::test]
async fn test_server_json_request_response() {
	let router = Arc::new(
		Router::new().function("/echo", Method::POST, |request| async move {
			let json: serde_json::Value = request.json()?;
			Response::ok().with_json(&json)
		}),
	);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();
	let test_data = serde_json::json!({
		"name": "Alice",
		"age": 30
	});

	let response = client
		.post(format!("{}/echo", server.url))
		.json(&test_data)
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::OK);
	let response_json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(response_json["name"], "Alice");
	assert_eq!(response_json["age"], 30);
}

#[tokio::test]
async fn test_server_404_response() {
	let router = Arc::new(
		Router::new().function("/exists", Method::GET, |_req| async {
			Ok(Response::ok().with_body("I exist!"))
		}),
	);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();
	let response = client
		.get(format!("{}/notfound", server.url))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_server_concurrent_requests() {
	let router = Arc::new(
		Router::new().function("/test", Method::GET, |request| async move {
			Ok(Response::ok().with_body(format!("Method: {}", request.method)))
		}),
	);

	let server = test_server_guard(router).await;

	// Make multiple concurrent requests
	let mut handles = vec![];
	for _ in 0..10 {
		let url = server.url.clone();
		let handle = tokio::spawn(async move {
			let client = reqwest::Client::new();
			client
				.get(format!("{}/test", url))
				.send()
				.await
				.unwrap()
				.text()
				.await
				.unwrap()
		});
		handles.push(handle);
	}

	// Wait for all requests to complete
	for handle in handles {
		let result = handle.await.unwrap();
		assert_eq!(result, "Method: GET");
	}
}

#[tokio::test]
async fn test_server_custom_headers() {
	let router = Arc::new(
		Router::new().function("/headers", Method::GET, |request| async move {
			let user_agent = request
				.headers
				.get(hyper::header::USER_AGENT)
				.and_then(|v| v.to_str().ok())
				.unwrap_or("Unknown");
			Ok(Response::ok().with_body(format!("User-Agent: {}", user_agent)))
		}),
	);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();
	let response = client
		.get(format!("{}/headers", server.url))
		.header("User-Agent", "TestAgent/1.0")
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::OK);
	assert_eq!(response.text().await.unwrap(), "User-Agent: TestAgent/1.0");
}

#[tokio::test]
async fn test_server_path_parameters() {
	let router =
		Arc::new(
			Router::new().function("/users/{id}", Method::GET, |request| async move {
				let id = request.path_params.get("id").unwrap();
				Ok(Response::ok().with_body(format!("ID: {}", id)))
			}),
		);

	let server = test_server_guard(router).await;

	let client = reqwest::Client::new();
	let response = client
		.get(format!("{}/users/123", server.url))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), reqwest::StatusCode::OK);
	assert_eq!(response.text().await.unwrap(), "ID: 123");
}
