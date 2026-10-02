use reinhardt_macros::url_patterns;

#[url_patterns]
fn local_binding() -> UnifiedRouter {
	let router = UnifiedRouter::new();
	router
}

#[url_patterns]
fn explicit_return() -> UnifiedRouter {
	return UnifiedRouter::new();
}

#[url_patterns]
fn empty() -> UnifiedRouter {}

#[url_patterns]
fn semicolon() -> UnifiedRouter {
	UnifiedRouter::new();
}

fn main() {}
