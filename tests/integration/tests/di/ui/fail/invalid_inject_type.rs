use reinhardt_di::{Depends, InjectionContext, SingletonScope};
use std::sync::Arc;

struct NonInjectableService {
	value: String,
}

// This should fail: NonInjectableService does not implement Injectable
#[tokio::main]
async fn main() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	// This line should fail: NonInjectableService is not Injectable
	let service = Depends::<NonInjectableService>::resolve(&ctx)
		.await
		.unwrap();
	println!("{}", service.value);
}
