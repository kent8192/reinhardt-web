// Compile-fail test: injectable on a non-async function

#![allow(unused_imports)] // Broad imports keep compile-fail diagnostics focused.

use reinhardt_di::{KeyedFactoryOutput, injectable};

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

#[derive(Clone)]
struct MyService;

#[injectable(scope = "singleton")]
fn make_service() -> KeyedFactoryOutput<MyServiceKey, MyService> {
	KeyedFactoryOutput::new(MyService)
}

fn main() {}
