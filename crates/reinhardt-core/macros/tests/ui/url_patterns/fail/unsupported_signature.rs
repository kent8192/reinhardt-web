use reinhardt_macros::url_patterns;

#[url_patterns]
async fn asynchronous() -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
unsafe fn unsafe_builder() -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
const fn constant_builder() -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
extern "C" fn external_builder() -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
fn parameter(value: u8) -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
fn generic<T>() -> UnifiedRouter {
	UnifiedRouter::new()
}

#[url_patterns]
fn constrained() -> UnifiedRouter
where
	(): Sized,
{
	UnifiedRouter::new()
}

fn main() {}
