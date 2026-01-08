use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Clone)]
struct ServiceA {
	b: Arc<ServiceB>,
}

#[async_trait::async_trait]
impl Injectable for ServiceA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let b = ServiceB::inject(ctx).await?;
		Ok(Self { b: Arc::new(b) })
	}
}

#[derive(Clone)]
struct ServiceB {
	a: Arc<ServiceA>,
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let a = ServiceA::inject(ctx).await?;
		Ok(Self { a: Arc::new(a) })
	}
}

// This should compile but fail at runtime with circular dependency detection
#[tokio::main]
async fn main() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// This will cause circular dependency at runtime
	let _service_a = ServiceA::inject(&ctx).await.unwrap();
}
