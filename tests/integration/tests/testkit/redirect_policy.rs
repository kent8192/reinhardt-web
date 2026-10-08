//! Redirect policy coverage across the test client, router, and network server.

use async_trait::async_trait;
use reinhardt_http::{Handler as HttpHandler, Request as HttpRequest, Response as HttpResponse};
use reinhardt_test::fixtures::test_server_guard;
use reinhardt_test::{APIClient, ClientError, RedirectPolicy};
use reinhardt_urls::routers::ServerRouter;
use rstest::{fixture, rstest};

struct RedirectHandler {
	response: HttpResponse,
}

#[async_trait]
impl HttpHandler for RedirectHandler {
	async fn handle(&self, _: HttpRequest) -> reinhardt_http::Result<HttpResponse> {
		Ok(self.response.clone())
	}
}

#[fixture]
fn redirect_router() -> ServerRouter {
	ServerRouter::new()
		.handler(
			"/a",
			RedirectHandler {
				response: HttpResponse::new(http::StatusCode::FOUND).with_header("Location", "/b"),
			},
		)
		.handler(
			"/b",
			RedirectHandler {
				response: HttpResponse::new(http::StatusCode::FOUND).with_header("Location", "/c"),
			},
		)
		.handler(
			"/c",
			RedirectHandler {
				response: HttpResponse::ok().with_body("redirect target"),
			},
		)
}

#[rstest]
#[tokio::test]
async fn redirect_never_returns_original_status_and_location(redirect_router: ServerRouter) {
	// Arrange
	let server = test_server_guard(redirect_router).await;
	let client = APIClient::builder()
		.base_url(&server.url)
		.redirect_policy(RedirectPolicy::Never)
		.build();

	// Act
	let response = client.get("/a").await.unwrap();

	// Assert
	assert_eq!(response.status(), http::StatusCode::FOUND);
	assert_eq!(response.header("Location"), Some("/b"));
}

#[rstest]
#[case::default(None)]
#[case::exact_limit(Some(RedirectPolicy::Follow { limit: 2 }))]
#[tokio::test]
async fn redirect_follow_returns_final_response(
	redirect_router: ServerRouter,
	#[case] policy: Option<RedirectPolicy>,
) {
	// Arrange
	let server = test_server_guard(redirect_router).await;
	let mut builder = APIClient::builder().base_url(&server.url);
	if let Some(policy) = policy {
		builder = builder.redirect_policy(policy);
	}
	let client = builder.build();

	// Act
	let response = client.get("/a").await.unwrap();

	// Assert
	assert_eq!(response.status(), http::StatusCode::OK);
	assert_eq!(response.body().as_ref(), b"redirect target");
	assert_eq!(response.header("Location"), None);
}

#[rstest]
#[case::zero_limit(0, "/b")]
#[case::two_hops_exceed_one(1, "/a")]
#[tokio::test]
async fn redirect_follow_limit_exceeded_is_redirect_error(
	redirect_router: ServerRouter,
	#[case] limit: usize,
	#[case] path: &str,
) {
	// Arrange
	let server = test_server_guard(redirect_router).await;
	let client = APIClient::builder()
		.base_url(&server.url)
		.redirect_policy(RedirectPolicy::Follow { limit })
		.build();

	// Act
	let error = match client.get(path).await {
		Err(error) => error,
		Ok(response) => panic!(
			"expected redirect failure, got status {}",
			response.status()
		),
	};

	// Assert
	assert!(error.is_redirect());
	assert!(matches!(error, ClientError::Reqwest(_)));
}

#[rstest]
#[case::from_handler(APIClient::from_handler)]
#[case::builder(|router| APIClient::builder()
	.redirect_policy(RedirectPolicy::Follow { limit: 0 })
	.handler(router)
	.build())]
#[tokio::test]
async fn redirect_in_process_returns_original_status_and_location(
	redirect_router: ServerRouter,
	#[case] create_client: fn(ServerRouter) -> APIClient,
) {
	// Arrange
	let client = create_client(redirect_router);

	// Act
	let response = client.get("/a").await.unwrap();

	// Assert
	assert_eq!(response.status(), http::StatusCode::FOUND);
	assert_eq!(response.header("Location"), Some("/b"));
}
