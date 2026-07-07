use reinhardt_di::{KeyedDepends, KeyedFactoryOutput, injectable, injectable_key};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[injectable_key]
pub struct ConfigKey;

#[derive(Clone)]
pub struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> KeyedFactoryOutput<ConfigKey, AppConfig> {
	KeyedFactoryOutput::new(AppConfig { value: "server_fn" })
}

#[server_fn]
pub async fn hello(
	#[inject] config: KeyedDepends<ConfigKey, AppConfig>,
) -> Result<String, ServerFnError> {
	Ok(config.value.to_string())
}

fn main() {}
