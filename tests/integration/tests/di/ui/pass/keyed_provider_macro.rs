use reinhardt_di::{KeyedDepends, KeyedFactoryOutput, injectable, injectable_key};

#[injectable_key]
struct ConfigKey;

#[derive(Clone)]
struct AppConfig {
	value: &'static str,
}

#[injectable(scope = "transient")]
async fn app_config() -> KeyedFactoryOutput<ConfigKey, AppConfig> {
	KeyedFactoryOutput::new(AppConfig { value: "ok" })
}

fn consume(config: KeyedDepends<ConfigKey, AppConfig>) -> &'static str {
	config.value
}

fn main() {}
