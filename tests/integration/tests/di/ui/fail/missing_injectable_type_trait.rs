use reinhardt_di::{FactoryOutput, InjectableKey, injectable};
use reinhardt_http::{Response, ViewResult};
use reinhardt_macros::get;

#[derive(Clone, Debug)]
struct AppConfig {
	host: String,
}

struct AppConfigKey;

impl InjectableKey for AppConfigKey {}

#[injectable(scope = "transient")]
async fn make_app_config() -> FactoryOutput<AppConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig {
		host: "localhost".to_string(),
	})
}

struct Broken<T>(reinhardt_di::Depends<AppConfigKey, T>)
where
	T: Send + Sync + 'static;

#[get("/broken-wrapper", name = "broken-wrapper")]
async fn broken_wrapper(#[inject] cfg: Broken<AppConfig>) -> ViewResult<Response> {
	Ok(Response::ok().with_body(cfg.0.host.clone()))
}

fn main() {}
