use reinhardt_di::{Depends, FactoryOutput, injectable, injectable_key};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[injectable_key]
pub struct ConfigKey;

#[derive(Clone)]
pub struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> FactoryOutput<ConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig { value: "server_fn" })
}

#[server_fn]
pub async fn hello(
	#[inject] config: Depends<ConfigKey, AppConfig>,
) -> Result<String, ServerFnError> {
	Ok(config.value.to_string())
}

fn main() {}
