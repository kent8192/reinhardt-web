use reinhardt_macros::url_patterns;

#[url_patterns(server)]
fn urls() -> UnifiedRouter {
	UnifiedRouter::new()
}

fn main() {}
