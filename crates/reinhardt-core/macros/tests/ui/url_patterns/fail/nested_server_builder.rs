use reinhardt_macros::url_patterns;

#[url_patterns]
fn merged() -> UnifiedRouter {
	UnifiedRouter::new().merge(UnifiedRouter::new().server(configure))
}

#[url_patterns]
fn aliased() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		let router = UnifiedRouter::new();
		router.server(configure)
	})
}

#[url_patterns]
fn mounted() -> UnifiedRouter {
	UnifiedRouter::new().mount_unified("/", UnifiedRouter::new().server(configure))
}

#[url_patterns]
fn client_argument() -> UnifiedRouter {
	UnifiedRouter::new().client(|client| {
		consume(UnifiedRouter::new().server(configure));
		client
	})
}

fn main() {}
