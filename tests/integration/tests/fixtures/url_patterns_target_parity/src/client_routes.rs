// The macro bridge exposes the shared type without enabling the Pages UI crate.
use reinhardt::reinhardt_types::page::Page;
use reinhardt::{ClientRouter, UnifiedRouter};

#[reinhardt::url_patterns]
fn login_urls() -> UnifiedRouter {
	UnifiedRouter::new()
		.client(|client| client.route("login", "/login/", || Page::Empty))
		.with_namespace("auth")
}

#[reinhardt::url_patterns]
pub fn client_pages() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(|server| server.endpoint(crate::native_handlers::page_health))
		.client({
			#[cfg(test)]
			crate::evaluation::record("client-argument");
			|client| client.route("home", "/", || Page::Empty)
		})
		.mount_unified("/ignored-native-prefix/", login_urls())
}

struct Other;

impl Other {
	fn server(self, client: ClientRouter) -> ClientRouter {
		client.route("extra", "/extra/", || Page::Empty)
	}
}

#[reinhardt::url_patterns]
pub fn unrelated_server_call() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(|server| server.endpoint(crate::native_handlers::page_health))
		.client(|client| Other.server(client))
}
