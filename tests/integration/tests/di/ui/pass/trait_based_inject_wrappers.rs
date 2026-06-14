//! Compile-pass test: trait-based `#[inject]` wrapper resolution.
//!
//! Regression guard for issue #4938. The route and server-function macros
//! must not identify wrapper parameters by literal type names; fully qualified
//! paths, renamed imports, type aliases, and custom `InjectableType` wrappers
//! all compile without requiring the inner type to implement `Injectable`.

use reinhardt_di::{Depends as Dep, InjectableType, injectable_factory};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

// Factory-registered type: deliberately no `impl Injectable`.
#[injectable_factory(scope = "transient")]
async fn make_app_config() -> AppConfig {
	AppConfig {
		host: "localhost".to_string(),
	}
}

type Alias<T> = reinhardt_di::Depends<T>;

struct Lazy<T>(reinhardt_di::Depends<T>)
where
	T: Send + Sync + 'static;

impl<T> InjectableType for Lazy<T>
where
	T: Send + Sync + 'static,
{
	type Inner = T;

	fn from_depends(depends: reinhardt_di::Depends<Self::Inner>) -> Self {
		Self(depends)
	}
}

#[get("/trait/fq", name = "trait-fq-depends")]
async fn route_fq(
	#[inject] cfg: reinhardt_di::Depends<AppConfig>,
) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/renamed", name = "trait-renamed-depends")]
async fn route_renamed(#[inject] cfg: Dep<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/alias", name = "trait-alias-depends")]
async fn route_alias(#[inject] cfg: Alias<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/custom", name = "trait-custom-wrapper")]
async fn route_custom(#[inject] cfg: Lazy<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.0.host.clone()))
}

#[server_fn]
async fn server_fq(
	#[inject] cfg: reinhardt_di::Depends<AppConfig>,
) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

#[server_fn]
async fn server_alias(#[inject] cfg: Alias<AppConfig>) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

#[server_fn]
async fn server_custom(#[inject] cfg: Lazy<AppConfig>) -> Result<String, ServerFnError> {
	Ok(cfg.0.host.clone())
}

fn main() {}
