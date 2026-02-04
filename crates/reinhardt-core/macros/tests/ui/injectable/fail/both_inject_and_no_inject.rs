//! Test that a field cannot have both `#[inject]` and `#[no_inject]`
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
	#[inject]
	#[no_inject] // Error: cannot have both
	db: Database,
}

fn main() {}
