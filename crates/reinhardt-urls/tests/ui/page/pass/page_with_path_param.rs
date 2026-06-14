//! Compile-pass: a Props struct with a `PathParam<T>` field, a manual
//! `FromRequest` impl, and a page-fn registered via
//! `ClientRouter::page`. Refs #4668 / P7 part 2.

use reinhardt_core::types::page::Page;
use reinhardt_urls::routers::ClientRouter;
use reinhardt_urls::routers::client_router::from_request::{
	ExtractError, FromRequest, PathParam, RouteContext,
};

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

fn main() {
	let _ = ClientRouter::new().page("user", "/users/{id}/", user_page);
}
