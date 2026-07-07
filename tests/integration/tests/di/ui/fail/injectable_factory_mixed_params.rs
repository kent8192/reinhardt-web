// Compile-fail test: injectable with mix of inject and non-inject params

#![allow(unused_imports)] // Broad imports keep compile-fail diagnostics focused.

use reinhardt_di::{KeyedDepends, injectable};

struct ConfigKey;

impl reinhardt_di::InjectableKey for ConfigKey {}

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

#[derive(Clone)]
struct MyService {
	name: String,
}

#[derive(Clone)]
struct Config;

// This should fail: cannot mix inject and non-inject params
#[injectable(scope = "transient")]
async fn make_service(
	#[inject] _config: KeyedDepends<ConfigKey, Config>,
	extra: String,
) -> MyService {
	MyService { name: extra }
}

fn main() {}
