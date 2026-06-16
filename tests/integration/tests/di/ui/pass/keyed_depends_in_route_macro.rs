use reinhardt_di::{Depends, FactoryOutput, injectable, injectable_key};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;

#[injectable_key]
struct ConfigKey;

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> FactoryOutput<ConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig { value: "route" })
}

#[get("/hello", name = "hello")]
async fn hello(
	#[inject] config: Depends<ConfigKey, AppConfig>,
) -> ViewResult<Response> {
	Ok(Response::ok().with_body(config.value.to_string()))
}

fn main() {}
