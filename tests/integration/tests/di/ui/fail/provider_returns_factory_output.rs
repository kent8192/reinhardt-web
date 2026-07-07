use reinhardt_di::{FactoryOutput, injectable, injectable_key};

#[derive(Clone)]
struct AppConfig;

#[injectable_key]
struct AppConfigKey;

#[injectable(scope = "request")]
async fn app_config() -> FactoryOutput<AppConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig)
}

fn main() {}
