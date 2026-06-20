use reinhardt_di::{Depends, FactoryOutput, injectable, injectable_key};

#[injectable_key]
struct ConfigKey;

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> FactoryOutput<ConfigKey, AppConfig> {
	FactoryOutput::new(AppConfig { value: "ok" })
}

fn consume(config: Depends<ConfigKey, AppConfig>) -> &'static str {
	config.value
}

fn main() {}
