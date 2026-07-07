use reinhardt_di::{
	DiResult, Injectable, InjectableKey, InjectionContext, KeyedDepends, KeyedFactoryOutput,
	injectable,
};

#[derive(Clone, Debug)]
struct MyService {
	value: String,
}

struct HandlerKey;

impl InjectableKey for HandlerKey {}

#[async_trait::async_trait]
impl Injectable for MyService {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(Self {
            value: "test".to_string(),
        })
    }
}

#[derive(Clone, Debug)]
struct MyConfig {
	host: String,
}

struct MyConfigKey;

impl InjectableKey for MyConfigKey {}

struct RouterKey;

impl InjectableKey for RouterKey {}

struct AppKey;

impl InjectableKey for AppKey {}

#[injectable(scope = "transient")]
async fn make_config() -> KeyedFactoryOutput<MyConfigKey, MyConfig> {
	KeyedFactoryOutput::new(MyConfig {
		host: "localhost".to_string(),
	})
}

#[async_trait::async_trait]
impl Injectable for MyConfig {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(Self {
            host: "localhost".to_string(),
        })
    }
}

// Case 1: #[inject] with non-Depends type (requires Clone)
#[injectable(scope = "transient")]
async fn make_handler(#[inject] service: MyService) -> KeyedFactoryOutput<HandlerKey, String> {
	KeyedFactoryOutput::new(service.value)
}

// Case 2: #[inject] with KeyedDepends<K, T> type
#[injectable(scope = "transient")]
async fn make_router(
	#[inject] config: KeyedDepends<MyConfigKey, MyConfig>,
) -> KeyedFactoryOutput<RouterKey, String> {
	KeyedFactoryOutput::new(config.host.clone())
}

// Case 3: Multiple #[inject] parameters mixing both patterns
#[injectable(scope = "transient")]
async fn make_app(
	#[inject] service: MyService,
	#[inject] config: KeyedDepends<MyConfigKey, MyConfig>,
) -> KeyedFactoryOutput<AppKey, String> {
	KeyedFactoryOutput::new(format!("{}:{}", config.host, service.value))
}

fn main() {}
