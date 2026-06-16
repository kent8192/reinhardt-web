//! Integration tests for the extractor-in-factory feature (Issue #4645).
//!
//! Exercises the new `Injectable` impls on `Path<T>`, `Query<T>`, `Json<T>`
//! (PR-1) and the `ParamContext` body cache that lets repeated `Json<T>`
//! resolutions in the same request share one read of the underlying body
//! (PR-2). Tests run against the `params` + `macros` feature combination,
//! which is what the proposed user API targets.

#![cfg(all(feature = "params", feature = "macros"))]

use bytes::Bytes;
use hyper::{HeaderMap, Method, Version, header};
use reinhardt_di::params::{Json, ParamContext, Path, Query};
use reinhardt_di::{
	DiError, FactoryOutput, Injectable, InjectableKey, InjectionContext, Request, SingletonScope,
	global_registry, injectable,
};
use reinhardt_http::PathParams;
use rstest::rstest;
use serde::Deserialize;
use serial_test::serial;
use std::sync::Arc;

// =============================================================================
// Helpers
// =============================================================================

fn build_request(method: Method, uri: &str, content_type: Option<&str>, body: &str) -> Request {
	let mut headers = HeaderMap::new();
	if let Some(ct) = content_type {
		headers.insert(header::CONTENT_TYPE, ct.parse().unwrap());
	}
	Request::builder()
		.method(method)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::copy_from_slice(body.as_bytes()))
		.build()
		.unwrap()
}

fn ctx_with_request(request: Request, params: PathParams) -> InjectionContext {
	let singleton = Arc::new(SingletonScope::new());
	InjectionContext::builder(singleton)
		.with_request(request)
		.with_param_context(ParamContext::with_path_params(params))
		.build()
}

// =============================================================================
// Path<T> Injectable
// =============================================================================

#[rstest]
#[tokio::test]
async fn path_i64_resolves_when_param_context_present() {
	// Arrange
	let mut params = PathParams::new();
	params.insert("id", "42");
	let req = build_request(Method::GET, "/q/42", None, "");
	let ctx = ctx_with_request(req, params);

	// Act
	let extracted = Path::<i64>::inject(&ctx).await;

	// Assert
	let Path(id) = extracted.expect("Path<i64> must resolve when ParamContext carries the param");
	assert_eq!(id, 42);
}

#[rstest]
#[tokio::test]
async fn path_string_resolves_when_param_context_present() {
	// Arrange
	let mut params = PathParams::new();
	params.insert("slug", "hello-world");
	let req = build_request(Method::GET, "/p/hello-world", None, "");
	let ctx = ctx_with_request(req, params);

	// Act
	let extracted = Path::<String>::inject(&ctx).await;

	// Assert
	let Path(slug) = extracted.expect("Path<String> must resolve when ParamContext carries it");
	assert_eq!(slug, "hello-world");
}

#[rstest]
#[tokio::test]
async fn path_returns_missing_param_context_when_absent() {
	// Arrange — build a context without `.with_param_context(...)`.
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Act
	let result = Path::<i64>::inject(&ctx).await;

	// Assert
	match result {
		Err(DiError::MissingParamContext { extractor }) => {
			assert_eq!(extractor, "Path");
		}
		other => panic!("expected DiError::MissingParamContext, got {:?}", other),
	}
}

// =============================================================================
// Query<T> Injectable
// =============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct Pagination {
	page: i32,
	per_page: i32,
}

#[rstest]
#[tokio::test]
async fn query_resolves_with_url_query_string() {
	// Arrange
	let req = build_request(Method::GET, "/list?page=3&per_page=25", None, "");
	let ctx = ctx_with_request(req, PathParams::new());

	// Act
	let extracted = Query::<Pagination>::inject(&ctx).await;

	// Assert
	let Query(pag) = extracted.expect("Query<T> must resolve from the URL query string");
	assert_eq!(
		pag,
		Pagination {
			page: 3,
			per_page: 25
		}
	);
}

// =============================================================================
// Json<T> Injectable + body cache (PR-2)
// =============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct BodyPayload {
	name: String,
}

#[rstest]
#[tokio::test]
async fn json_resolves_with_application_json_body() {
	// Arrange
	let req = build_request(
		Method::POST,
		"/api/items",
		Some("application/json"),
		r#"{"name":"alice"}"#,
	);
	let ctx = ctx_with_request(req, PathParams::new());

	// Act
	let extracted = Json::<BodyPayload>::inject(&ctx).await;

	// Assert
	let Json(payload) = extracted.expect("Json<T> must resolve when Content-Type is JSON");
	assert_eq!(
		payload,
		BodyPayload {
			name: "alice".into()
		}
	);
}

#[rstest]
#[tokio::test]
async fn json_can_be_resolved_twice_in_the_same_request_via_body_cache() {
	// Arrange — the same single body, two separate Json<T> resolutions.
	// Without PR-2's ParamContext body cache, the second call would fail
	// with "Request body has already been consumed".
	let req = build_request(
		Method::POST,
		"/api/items",
		Some("application/json"),
		r#"{"name":"bob"}"#,
	);
	let ctx = ctx_with_request(req, PathParams::new());

	// Act
	let first = Json::<BodyPayload>::inject(&ctx).await;
	let second = Json::<BodyPayload>::inject(&ctx).await;

	// Assert — both must succeed with identical bytes.
	let Json(p1) = first.expect("first Json<T>::inject must succeed");
	let Json(p2) = second.expect("second Json<T>::inject must succeed because body is cached");
	assert_eq!(p1, BodyPayload { name: "bob".into() });
	assert_eq!(p2, p1);
}

// =============================================================================
// `#[injectable]` provider body uses extractors via `#[inject]`
//
// This is the user-facing API from Issue #4645: a factory composes over an
// `#[inject] Path(id): Path<i64>` and resolves at runtime through the macro's
// registry-first `Injectable::inject` fallback path.
// =============================================================================

#[derive(Clone, Debug, PartialEq)]
struct AuthoredItem {
	id: i64,
}

struct AuthoredItemKey;

impl InjectableKey for AuthoredItemKey {}

#[injectable(scope = "request")]
async fn authored_item(#[inject] path: Path<i64>) -> FactoryOutput<AuthoredItemKey, AuthoredItem> {
	FactoryOutput::new(AuthoredItem { id: path.0 })
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_factory_can_consume_path_extractor() {
	// Arrange
	let mut params = PathParams::new();
	params.insert("id", "7");
	let req = build_request(Method::GET, "/items/7", None, "");
	let ctx = ctx_with_request(req, params);

	// Act — resolving the keyed factory output triggers the provider, which itself
	// resolves `Path<i64>` through the new Injectable impl.
	let resolved = ctx
		.resolve::<FactoryOutput<AuthoredItemKey, AuthoredItem>>()
		.await;

	// Assert
	let item = resolved.expect("factory resolution must succeed end-to-end");
	assert_eq!(item.as_ref().as_ref(), &AuthoredItem { id: 7 });
	assert!(
		global_registry().is_registered::<FactoryOutput<AuthoredItemKey, AuthoredItem>>(),
		"the factory registration submitted via inventory must be live"
	);
}

// Combined extractor + Injectable dependency — proves that the macro's
// `Injectable::inject` fallback path keeps working alongside the new
// extractor `Injectable` impls.
//
// `AppContext` exposes its DI surface through a manual `impl Injectable`
// only (it is never registered in the global registry). Per the
// `#[injectable]` provider contract (kent8192/reinhardt-web#4685), a
// manually-Injectable, unregistered type MUST be requested via the
// non-`Depends` `#[inject] T` form, which resolves through the registry-first
// `T::inject` fallback. The `Depends<K, T>` form is reserved for
// factory-produced types that resolve via keyed factory output — see
// `injectable_factory_inject_fallback_tests.rs`.

#[derive(Clone, Debug, PartialEq)]
struct AppContext {
	tag: &'static str,
}

#[async_trait::async_trait]
impl Injectable for AppContext {
	async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(AppContext { tag: "global" })
	}
}

#[derive(Clone, Debug, PartialEq)]
struct AuthoredItemWithCtx {
	id: i64,
	tag: &'static str,
}

struct AuthoredItemWithCtxKey;

impl InjectableKey for AuthoredItemWithCtxKey {}

#[injectable(scope = "request")]
async fn authored_item_with_ctx(
	#[inject] path: Path<i64>,
	#[inject] app: AppContext,
) -> FactoryOutput<AuthoredItemWithCtxKey, AuthoredItemWithCtx> {
	FactoryOutput::new(AuthoredItemWithCtx {
		id: path.0,
		tag: app.tag,
	})
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_factory_mixes_path_extractor_and_depends() {
	// Arrange
	let mut params = PathParams::new();
	params.insert("id", "99");
	let req = build_request(Method::GET, "/items/99", None, "");
	let ctx = ctx_with_request(req, params);

	// Act
	let resolved = ctx
		.resolve::<FactoryOutput<AuthoredItemWithCtxKey, AuthoredItemWithCtx>>()
		.await;

	// Assert
	let item = resolved.expect("mixed factory must succeed end-to-end");
	assert_eq!(
		item.as_ref().as_ref(),
		&AuthoredItemWithCtx {
			id: 99,
			tag: "global"
		}
	);
}
