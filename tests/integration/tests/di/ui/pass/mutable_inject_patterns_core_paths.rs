//! Compile-pass coverage for mutable injected patterns in core macro paths.

use async_trait::async_trait;
use reinhardt_di::{DiResult, Injectable, InjectionContext};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::{get, routes, use_inject};
use reinhardt_urls::routers::UnifiedRouter;

#[derive(Clone)]
struct Database;

#[derive(Clone)]
struct Data;

#[derive(Clone)]
struct Wrapper<T>(T);

#[async_trait]
impl Injectable for Database {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self)
	}
}

#[async_trait]
impl Injectable for Wrapper<Data> {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self(Data))
	}
}

fn borrow_mutably<T>(_: &mut T) {}

#[get("/mutable", use_inject = true)]
async fn mutable_route_handler(
	#[inject] mut db: Database,
	#[inject] Wrapper(mut value): Wrapper<Data>,
) -> ViewResult<Response> {
	borrow_mutably(&mut db);
	borrow_mutably(&mut value);
	Ok(Response::ok())
}

#[use_inject]
async fn mutable_use_inject_handler(
	#[inject] mut db: Database,
	#[inject] Wrapper(mut value): Wrapper<Data>,
) -> ViewResult<Response> {
	borrow_mutably(&mut db);
	borrow_mutably(&mut value);
	Ok(Response::ok())
}

#[routes]
async fn mutable_routes_registration(
	#[inject] mut db: Database,
	#[inject] Wrapper(mut value): Wrapper<Data>,
) -> UnifiedRouter {
	borrow_mutably(&mut db);
	borrow_mutably(&mut value);
	UnifiedRouter::new()
}

fn main() {}
