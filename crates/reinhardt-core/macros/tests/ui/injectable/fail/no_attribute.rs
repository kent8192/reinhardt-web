//! Test that fields must have either `#[inject]` or `#[no_inject]`
//!
//! This should fail to compile with an error message

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

#[injectable]
#[derive(Clone)]
struct MyService {
	db: Database, // Error: must have #[inject] or #[no_inject]
}

fn main() {}
