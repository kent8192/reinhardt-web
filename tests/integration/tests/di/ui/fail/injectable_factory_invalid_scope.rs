// Compile-fail test: injectable with invalid scope value

#![allow(unused_imports)] // Broad imports keep compile-fail diagnostics focused.

use reinhardt_di::{FactoryOutput, injectable};

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

#[derive(Clone)]
struct MyService;

#[injectable(scope = "invalid_scope")]
async fn make_service() -> FactoryOutput<MyServiceKey, MyService> {
	FactoryOutput::new(MyService)
}

fn main() {}
