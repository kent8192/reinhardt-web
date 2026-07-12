//! Compile-pass test: `KeyedDepends<K, ProviderType>` in `#[server_fn]`.
//!
//! Regression guard for issue #3723 (fixed in PR #3725); test coverage
//! added via issue #3727. The `#[server_fn]` macro routes `KeyedDepends<K, T>`
//! parameters through `resolve_from_registry()`, which has no
//! `T: Injectable` bound. This allows keyed provider-produced types registered
//! via `#[injectable]` to be injected without a manual
//! `Injectable` implementation. A regression that reintroduces the
//! `Injectable` bound on the `KeyedDepends<K, T>` codegen path would cause
//! this test to fail to compile.

use reinhardt_di::{InjectableKey, KeyedDepends, KeyedFactoryOutput, injectable};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

struct AppConfigKey;

impl InjectableKey for AppConfigKey {}

// Keyed provider-registered type: deliberately no `impl Injectable`.
#[injectable(scope = "transient")]
async fn make_app_config() -> KeyedFactoryOutput<AppConfigKey, AppConfig> {
	KeyedFactoryOutput::new(AppConfig {
		host: "localhost".into(),
	})
}

#[server_fn]
async fn get_host(
	#[inject] cfg: KeyedDepends<AppConfigKey, AppConfig>,
) -> Result<String, ServerFnError> {
	Ok(cfg.host.clone())
}

fn main() {}
