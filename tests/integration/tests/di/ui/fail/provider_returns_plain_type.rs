use reinhardt_di::injectable;

struct AppConfig;

#[injectable(scope = "transient")]
async fn app_config() -> AppConfig {
	AppConfig
}

fn main() {}
