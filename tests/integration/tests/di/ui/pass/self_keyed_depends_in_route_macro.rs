use reinhardt::get;
use reinhardt::{Response, ViewResult};
use reinhardt_di::{Depends, injectable};

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "request")]
async fn app_config() -> AppConfig {
	AppConfig { value: "route" }
}

#[get("/config", use_inject = true)]
async fn config_view(#[inject] config: Depends<AppConfig>) -> ViewResult<Response> {
	let _value = config.value;
	Ok(Response::ok())
}

fn main() {
	let _ = app_config;
	let _ = config_view;
}
