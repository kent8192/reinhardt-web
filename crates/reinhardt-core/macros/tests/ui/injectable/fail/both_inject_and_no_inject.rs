//! Test that a field cannot have both `#[inject]` and `#[no_inject]`
//!
//! This should fail to compile with an error message

use reinhardt_macros::injectable;

#[derive(Clone, Default)]
struct Database;

#[async_trait::async_trait]
impl reinhardt_di::Injectable for Database {
	async fn inject(_ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(Database)
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
