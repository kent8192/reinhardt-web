// Compile-fail test: injectable with non-inject parameters

#![allow(unused_imports)] // Broad imports keep compile-fail diagnostics focused.

use reinhardt_di::injectable;

#[derive(Clone)]
struct MyService {
	name: String,
}

#[injectable(scope = "singleton")]
async fn make_service(name: String) -> MyService {
	MyService { name }
}

fn main() {}
