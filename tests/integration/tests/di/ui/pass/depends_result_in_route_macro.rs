//! Compile-pass test: `KeyedDepends<K, Result<T, E>>` in `#[get]` handler.
//!
//! Regression guard for issue #4937. The `#[get]` route macro must route
//! keyed `KeyedDepends<K, Result<T, E>>` through `resolve_from_registry()`
//! against the expanded output type `KeyedFactoryOutput<K, Result<T, E>>` and
//! never add a `T: Injectable` bound. The inner
//! `Result<AppConfig, ConfigError>` is provider-produced and deliberately
//! has no `Injectable` impl, so any regression in the macro's alias handling
//! causes this test to fail to compile.

use reinhardt_di::{InjectableKey, KeyedDepends, KeyedFactoryOutput, injectable};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

#[derive(Clone, Debug)]
struct ConfigError;

struct AppConfigResultKey;

impl InjectableKey for AppConfigResultKey {}

// Keyed provider-registered Result type: deliberately no `impl Injectable`.
#[injectable(scope = "transient")]
async fn make_app_config()
-> KeyedFactoryOutput<AppConfigResultKey, Result<AppConfig, ConfigError>> {
	KeyedFactoryOutput::new(Ok(AppConfig {
		host: "localhost".to_string(),
	}))
}

#[get("/hello", name = "hello-depends-result")]
async fn hello(
	#[inject] cfg: KeyedDepends<AppConfigResultKey, Result<AppConfig, ConfigError>>,
) -> ViewResult<Response> {
	match &*cfg {
		Ok(c) => Ok(Response::ok().with_body(c.host.clone())),
		Err(_) => Ok(Response::ok().with_body("config error".to_string())),
	}
}

fn main() {}
