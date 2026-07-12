use reinhardt::pages::server_fn::{ServerFnError, server_fn};
use reinhardt_di::{Depends, injectable};

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "request")]
async fn app_config() -> AppConfig {
	AppConfig { value: "server_fn" }
}

#[server_fn]
async fn get_config(#[inject] config: Depends<AppConfig>) -> Result<String, ServerFnError> {
	Ok(config.value.to_string())
}

fn main() {
	let _ = app_config;
	let _ = get_config;
}
