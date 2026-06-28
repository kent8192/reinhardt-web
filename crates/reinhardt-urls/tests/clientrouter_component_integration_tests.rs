//! Integration tests for route-backed components registered via
//! `ClientRouter::component`.

#![cfg(feature = "client-router")]

use std::collections::HashMap;

use reinhardt_core::types::page::Page;
use reinhardt_urls::routers::ClientRouter;
use reinhardt_urls::routers::client_router::{
	ComponentInfo, ExtractError, FromRequest, PathParam, RouteContext,
};

#[derive(Debug)]
struct UserPageProps {
	id: i64,
}

impl FromRequest for UserPageProps {
	fn from_request(ctx: &RouteContext) -> Result<Self, ExtractError> {
		Ok(Self {
			id: PathParam::<i64>::extract(ctx, "id")?.into_inner(),
		})
	}
}

impl ComponentInfo for UserPageProps {
	fn path() -> &'static str {
		"/users/{id}/"
	}

	fn name() -> &'static str {
		"user-detail"
	}

	fn component_name() -> &'static str {
		"UserPage"
	}

	fn function_name() -> &'static str {
		"user_page"
	}

	fn props_type_name() -> &'static str {
		"UserPageProps"
	}
}

fn user_page(props: UserPageProps) -> Page {
	Page::Text(format!("user {}", props.id).into())
}

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

#[test]
fn component_registers_route_from_props_metadata() {
	let router = ClientRouter::new().component(user_page);

	let view = router_render(&router, "/users/7/");

	assert_eq!(page_text(&view), "user 7");
}

#[test]
fn component_surfaces_extract_error_like_page() {
	let router = ClientRouter::new().component(user_page);

	let view = router_render(&router, "/users/not-number/");

	assert_eq!(
		page_text(&view),
		"route extraction error on `/users/{id}/`: failed to parse `id`: invalid digit found in string",
	);
}

#[test]
fn generated_props_can_still_be_used_directly() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "9".to_string());
	let ctx = RouteContext::new("/users/9/".to_string(), params, "".to_string());

	let props = UserPageProps::from_request(&ctx).unwrap();
	let view = user_page(props);

	assert_eq!(page_text(&view), "user 9");
}
