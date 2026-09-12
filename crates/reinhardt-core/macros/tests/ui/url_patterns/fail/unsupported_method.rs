use reinhardt_macros::url_patterns;

#[url_patterns]
fn websocket() -> UnifiedRouter {
	UnifiedRouter::new().websocket(configure)
}

#[url_patterns]
fn grpc() -> UnifiedRouter {
	UnifiedRouter::new().grpc(configure)
}

#[url_patterns]
fn custom() -> UnifiedRouter {
	UnifiedRouter::new().custom(configure)
}

fn main() {}
