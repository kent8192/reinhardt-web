//! Integration tests for `ClientRouter::page<F, P>(pattern, handler)`
//! where `P: FromRequest`. Spec §4.3 / Refs #4668 / P7 part 2.

use rstest::rstest;

use reinhardt_core::types::page::Page;
use reinhardt_urls::routers::ClientRouter;
use reinhardt_urls::routers::client_router::from_request::{
	ExtractError, FromRequest, PathParam, QueryParam, RouteContext,
};

// --- Test Props + handler used by every test case --------------------------

#[derive(Debug)]
struct UserPageProps {
	id: PathParam<i32>,
}

impl FromRequest for UserPageProps {
	fn from_request(ctx: &RouteContext) -> Result<Self, ExtractError> {
		Ok(Self {
			id: PathParam::extract(ctx, "id")?,
		})
	}
}

fn user_page(props: UserPageProps) -> Page {
	Page::Text(format!("user {}", props.id.into_inner()).into())
}

#[derive(Debug)]
struct SearchPageProps {
	q: QueryParam<String>,
}

impl FromRequest for SearchPageProps {
	fn from_request(ctx: &RouteContext) -> Result<Self, ExtractError> {
		Ok(Self {
			q: QueryParam::extract(ctx, "q")?,
		})
	}
}

fn search_page(props: SearchPageProps) -> Page {
	Page::Text(format!("search: {}", props.q.into_inner()).into())
}

// --- Helpers ---------------------------------------------------------------

/// Render the router for a path by driving the public `current_path` signal
/// then invoking `render_current` — the same flow used by the existing
/// `test_render_current_returns_page_without_not_found` test in
/// `crates/reinhardt-urls/src/routers/client_router/core.rs`.
fn router_render(router: &ClientRouter, path: &str) -> Page {
	router.current_path().set(path.to_string());
	router.render_current()
}

fn page_text(page: &Page) -> String {
	match page {
		Page::Text(t) => t.to_string(),
		other => format!("{other:?}"),
	}
}

// --- Tests -----------------------------------------------------------------

#[rstest]
fn page_method_registers_and_renders_with_path_param() {
	// Arrange
	let router = ClientRouter::new().page("user-detail", "/users/{id}/", user_page);

	// Act
	let view = router_render(&router, "/users/42/");

	// Assert
	let s = page_text(&view);
	assert_eq!(s, "user 42", "expected `user 42`, got: {s}");
}

#[rstest]
fn page_method_extracts_query_param() {
	// Arrange
	let router = ClientRouter::new().page("search", "/search/", search_page);

	// Act — drive with a query string. The router's `match_path` ignores
	// the `?...` suffix; the extractor parses it from `RouteContext::query`.
	let view = router_render(&router, "/search/?q=rust");

	// Assert
	let s = page_text(&view);
	assert_eq!(s, "search: rust", "expected `search: rust`, got: {s}");
}

#[rstest]
fn page_method_surfaces_extract_error_as_page_text() {
	// Arrange — path matches but `id` cannot parse as i32
	let router = ClientRouter::new().page("user-detail-err", "/users/{id}/", user_page);

	// Act
	let view = router_render(&router, "/users/abc/");

	// Assert — the router surfaces the error as a Page::Text. The exact
	// formatting is part of the public contract.
	let s = page_text(&view);
	assert_eq!(
		s,
		"route extraction error on `/users/{id}/`: failed to parse `id`: invalid digit found in string",
	);
}

#[rstest]
fn page_method_returns_not_found_for_unmatched_path() {
	// Arrange
	let router = ClientRouter::new().page("user-detail-nf", "/users/{id}/", user_page);

	// Act
	let view = router_render(&router, "/nope/");

	// Assert — no not_found registered → Page::Empty fallback
	assert!(matches!(view, Page::Empty));
}

// --- Spec §4.3 last paragraph: "every page is a component" ----------------

#[rstest]
fn props_struct_can_be_constructed_directly_for_component_use() {
	// Arrange — build the same RouteContext the router would produce
	let mut params = std::collections::HashMap::new();
	params.insert("id".to_string(), "7".to_string());
	let ctx = RouteContext::new("/users/7/".to_string(), params, "".to_string());

	// Act — construct Props via FromRequest, then call `user_page` as a
	// plain function (i.e. as a component) and also via the registered
	// route (i.e. as a page handler).
	let props = UserPageProps::from_request(&ctx).expect("FromRequest must succeed");
	let view_as_component = user_page(props);

	let router = ClientRouter::new().page("user-detail-component", "/users/{id}/", user_page);
	let view_as_page = router_render(&router, "/users/7/");

	// Assert — both paths render identically. This is spec §4.3's
	// "same Props struct = page function = component" invariant.
	let a = page_text(&view_as_component);
	let b = page_text(&view_as_page);
	assert_eq!(a, "user 7");
	assert_eq!(b, "user 7");
	assert_eq!(a, b);
}
