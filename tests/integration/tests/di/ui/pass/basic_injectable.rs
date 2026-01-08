use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Clone)]
struct SimpleService {
	value: String,
}

#[async_trait::async_trait]
impl Injectable for SimpleService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			value: "test".to_string(),
		})
	}
}

#[tokio::main]
async fn main() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let service = SimpleService::inject(&ctx).await.unwrap();
	assert_eq!(service.value, "test");
}
