//! Compile-pass test: `Depends<K, Result<T, E>>` in `#[server_fn]`.
//!
//! Regression guard for issue #4937. The `#[server_fn]` macro must route
//! keyed `Depends<K, Result<T, E>>` through `resolve_from_registry()` against
//! the expanded output type `FactoryOutput<K, Result<T, E>>` and never add a
//! `T: Injectable` bound. The inner
//! `Result<AppConfig, ConfigError>` is factory-produced and deliberately
//! has no `Injectable` impl, so any regression in the macro's alias handling
//! causes this test to fail to compile.

use reinhardt_di::{Depends, FactoryOutput, InjectableKey, injectable};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

#[derive(Clone, Debug)]
struct ConfigError;

struct AppConfigResultKey;

impl InjectableKey for AppConfigResultKey {}

// Factory-registered Result type: deliberately no `impl Injectable`.
#[injectable(scope = "transient")]
async fn make_app_config() -> FactoryOutput<AppConfigResultKey, Result<AppConfig, ConfigError>> {
	FactoryOutput::new(Ok(AppConfig {
		host: "localhost".to_string(),
	}))
}

#[server_fn]
async fn get_host(
	#[inject] cfg: Depends<AppConfigResultKey, Result<AppConfig, ConfigError>>,
) -> Result<String, ServerFnError> {
	match &*cfg {
		Ok(c) => Ok(c.host.clone()),
		Err(_) => Ok("config error".to_string()),
	}
}

fn main() {}
