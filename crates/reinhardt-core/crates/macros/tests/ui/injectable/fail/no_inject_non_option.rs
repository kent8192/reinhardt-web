//! Test that `#[no_inject]` without default requires `Option<T>`
//!
//! This should fail to compile because the field type is not `Option<T>`

use reinhardt_macros::injectable;

#[derive(Clone, Default)]
struct Database;

#[async_trait::async_trait]
impl reinhardt_di::Injectable for Database {
	async fn inject(_ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(Database)
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
