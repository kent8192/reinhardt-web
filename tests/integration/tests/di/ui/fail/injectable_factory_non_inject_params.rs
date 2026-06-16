// Compile-fail test: injectable with non-inject parameters

#![allow(unused_imports)] // Broad imports keep compile-fail diagnostics focused.

use reinhardt_di::{FactoryOutput, injectable};

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

#[derive(Clone)]
struct MyService {
	name: String,
}

#[injectable(scope = "singleton")]
async fn make_service(name: String) -> FactoryOutput<MyServiceKey, MyService> {
	FactoryOutput::new(MyService { name })
}

fn main() {}
