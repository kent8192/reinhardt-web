use reinhardt_macros::url_patterns;

#[url_patterns]
fn empty() -> UnifiedRouter {
	UnifiedRouter::new().server()
}

#[url_patterns]
fn multiple() -> UnifiedRouter {
	UnifiedRouter::new().server(first, second)
}

#[url_patterns]
fn generic() -> UnifiedRouter {
	UnifiedRouter::new().server::<()>(configure)
}

fn main() {}
