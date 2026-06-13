//! Compile-pass test: `DependsResult<T, E>` in `#[server_fn]`.
//!
//! Regression guard for issue #4937. `DependsResult<T, E>` is sugar for
//! `Depends<Result<T, E>>`. The `#[server_fn]` macro must recognize the
//! alias and route it through `resolve_from_registry()` against the
//! expanded inner type `Result<T, E>` — never wrapping the alias in another
//! `Depends<...>` and never adding a `T: Injectable` bound. The inner
//! `Result<AppConfig, ConfigError>` is factory-produced and deliberately
//! has no `Injectable` impl, so any regression in the macro's alias handling
//! causes this test to fail to compile.

use reinhardt_di::{DependsResult, injectable_factory};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

#[derive(Clone, Debug)]
struct ConfigError;

// Factory-registered Result type: deliberately no `impl Injectable`.
#[injectable_factory(scope = "transient")]
async fn make_app_config() -> Result<AppConfig, ConfigError> {
	Ok(AppConfig {
		host: "localhost".to_string(),
	})
}

#[server_fn]
async fn get_host(
	#[inject] cfg: DependsResult<AppConfig, ConfigError>,
) -> Result<String, ServerFnError> {
	match &*cfg {
		Ok(c) => Ok(c.host.clone()),
		Err(_) => Ok("config error".to_string()),
	}
}

fn main() {}
