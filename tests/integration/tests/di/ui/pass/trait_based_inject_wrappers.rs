//! Compile-pass test: trait-based `#[inject]` wrapper resolution.
//!
//! Regression guard for issue #4938. The route and server-function macros
//! must not identify wrapper parameters by literal type names; fully qualified
//! paths, renamed imports, type aliases, and custom `InjectableType` wrappers
//! all compile without requiring the inner type to implement `Injectable`.

use reinhardt_di::{
	Depends as Dep, FactoryOutput, InjectableKey, InjectableType, injectable,
};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

struct AppConfigKey;

impl InjectableKey for AppConfigKey {}

#[injectable(scope = "transient")]
async fn make_keyed_app_config() -> FactoryOutput<AppConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig {
		host: "localhost".to_string(),
	})
}

type Alias<T> = reinhardt_di::Depends<AppConfigKey, T>;

struct Lazy<T>
where
	T: Send + Sync + 'static,
{
	inner: Arc<T>,
}

impl<T> std::ops::Deref for Lazy<T>
where
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<T> InjectableType for Lazy<T>
where
	T: Clone + Send + Sync + 'static,
{
	type Inner = FactoryOutput<AppConfigKey, T>;

	fn from_resolved(inner: Arc<Self::Inner>, _use_cache: bool) -> Self {
		Self {
			inner: Arc::new(inner.as_ref().as_ref().clone()),
		}
	}
}

#[get("/trait/fq", name = "trait-fq-depends")]
async fn route_fq(
	#[inject] cfg: reinhardt_di::Depends<AppConfigKey, AppConfig>,
) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/renamed", name = "trait-renamed-depends")]
async fn route_renamed(#[inject] cfg: Dep<AppConfigKey, AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/alias", name = "trait-alias-depends")]
async fn route_alias(#[inject] cfg: Alias<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[get("/trait/custom", name = "trait-custom-wrapper")]
async fn route_custom(#[inject] cfg: Lazy<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

#[server_fn]
async fn server_fq(
	#[inject] cfg: reinhardt_di::Depends<AppConfigKey, AppConfig>,
) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

#[server_fn]
async fn server_alias(#[inject] cfg: Alias<AppConfig>) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

#[server_fn]
async fn server_custom(#[inject] cfg: Lazy<AppConfig>) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

fn main() {}
