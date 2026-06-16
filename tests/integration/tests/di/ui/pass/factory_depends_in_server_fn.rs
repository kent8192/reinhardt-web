//! Compile-pass test: `Depends<K, FactoryType>` in `#[server_fn]`.
//!
//! Regression guard for issue #3723 (fixed in PR #3725); test coverage
//! added via issue #3727. The `#[server_fn]` macro routes `Depends<K, T>`
//! parameters through `resolve_from_registry()`, which has no
//! `T: Injectable` bound. This allows factory-produced types registered
//! via `#[injectable]` to be injected without a manual
//! `Injectable` implementation. A regression that reintroduces the
//! `Injectable` bound on the `Depends<K, T>` codegen path would cause this
//! test to fail to compile.

use reinhardt_di::{Depends, FactoryOutput, InjectableKey, injectable};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

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

#[server_fn]
async fn get_host(#[inject] cfg: Depends<AppConfigKey, AppConfig>) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

fn main() {}
