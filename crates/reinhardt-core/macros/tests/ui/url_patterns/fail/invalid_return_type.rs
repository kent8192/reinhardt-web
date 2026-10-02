use reinhardt_macros::url_patterns;

#[url_patterns]
fn missing() {
	UnifiedRouter::new()
}

#[url_patterns]
fn wrong_router() -> ServerRouter {
	UnifiedRouter::new()
}

#[url_patterns]
fn aliased() -> AppRouter {
	UnifiedRouter::new()
}

fn main() {}
