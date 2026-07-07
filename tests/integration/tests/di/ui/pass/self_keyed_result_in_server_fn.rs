use reinhardt::pages::server_fn::{ServerFnError, server_fn};
use reinhardt_di::{Depends, injectable};

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[derive(Clone)]
struct ConfigError;

#[injectable(scope = "request")]
async fn app_config() -> Result<AppConfig, ConfigError> {
	Ok(AppConfig { value: "checked" })
}

#[server_fn]
async fn get_config(
	#[inject] config: Depends<Result<
		AppConfig,
		ConfigError,
	>>,
) -> Result<String, ServerFnError> {
	let config = config
		.as_ref()
		.as_ref()
		.map_err(|_| ServerFnError::ServerError("config".into()))?;
	Ok(config.value.to_string())
}

fn main() {
	let _ = app_config;
	let _ = get_config;
}
