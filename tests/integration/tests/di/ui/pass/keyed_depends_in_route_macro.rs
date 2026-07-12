use reinhardt::get;
use reinhardt::http::Response;
use reinhardt::ViewResult;
use reinhardt_di::{KeyedDepends, KeyedFactoryOutput, injectable, injectable_key};

#[injectable_key]
struct ConfigKey;

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> KeyedFactoryOutput<ConfigKey, AppConfig> {
	KeyedFactoryOutput::new(AppConfig { value: "route" })
}

#[get("/hello", name = "hello")]
async fn hello(#[inject] config: KeyedDepends<ConfigKey, AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(config.value.to_string()))
}

fn main() {}
