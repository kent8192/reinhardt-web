//! Test that `#[no_inject]` without default requires `Option<T>`
//!
//! This should fail to compile because the field type is not `Option<T>`

use reinhardt_macros::injectable;

/// Mock Injectable trait for testing
pub trait Injectable {
	fn resolve() -> Self;
}

#[derive(Clone, Default)]
struct Database;

#[async_trait::async_trait]
impl Injectable for Database {
	fn resolve() -> Self {
		Database
	}
}

#[derive(Clone)]
struct Cache;

#[injectable]
#[derive(Clone)]
struct MyService {
	#[inject]
	db: Database,

	#[no_inject] // Error: must be Option<T>
	cache: Cache,
}

fn main() {}
