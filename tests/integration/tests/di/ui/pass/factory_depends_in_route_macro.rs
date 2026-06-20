//! Compile-pass test: `Depends<K, FactoryType>` in `#[get]` handler.
//!
//! Regression guard for issue #3723 (fixed in PR #3725). The `#[get]`
//! route macro routes `Depends<K, T>` parameters through
//! `resolve_from_registry()`, which has no `T: Injectable` bound. This
//! allows factory-produced types registered via `#[injectable]`
//! to be injected without a manual `Injectable` implementation. A
//! regression that reintroduces the `Injectable` bound on the
//! `Depends<K, T>` codegen path would cause this test to fail to compile.

use reinhardt_di::{Depends, FactoryOutput, InjectableKey, injectable};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

struct AppConfigKey;

impl InjectableKey for AppConfigKey {}

// Factory-registered type: deliberately no `impl Injectable`.
#[injectable(scope = "transient")]
async fn make_app_config() -> FactoryOutput<AppConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig {
		host: "localhost".to_string(),
	})
}

#[get("/hello", name = "hello-factory-depends")]
async fn hello(#[inject] cfg: Depends<AppConfigKey, AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.host.clone()))
}

fn main() {}
