use reinhardt_macros::url_patterns;

#[url_patterns]
fn helper_call() -> UnifiedRouter {
	make_router()
}

#[url_patterns]
fn variable() -> UnifiedRouter {
	router
}

#[url_patterns]
fn conditional() -> UnifiedRouter {
	if enabled {
		UnifiedRouter::new()
	} else {
		UnifiedRouter::default()
	}
}

#[url_patterns]
fn matching() -> UnifiedRouter {
	match enabled {
		_ => UnifiedRouter::new(),
	}
}

fn main() {}
