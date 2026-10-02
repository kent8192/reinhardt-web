use reinhardt_di::{Depends, KeyedDepends, KeyedFactoryOutput, injectable, injectable_key};

#[derive(Clone)]
struct AppConfig {
	host: &'static str,
}

#[injectable(scope = "request")]
async fn app_config() -> AppConfig {
	AppConfig { host: "self" }
}

fn consume_self_keyed(config: Depends<AppConfig>) -> &'static str {
	config.host
}

#[injectable_key]
struct PrimaryConfig;

#[injectable(scope = "request")]
async fn primary_config() -> KeyedFactoryOutput<PrimaryConfig, AppConfig> {
	KeyedFactoryOutput::new(AppConfig { host: "primary" })
}

fn consume_keyed(config: KeyedDepends<PrimaryConfig, AppConfig>) -> &'static str {
	config.host
}

fn main() {
	let _ = app_config;
	let _ = primary_config;
	let _ = consume_self_keyed;
	let _ = consume_keyed;
}
