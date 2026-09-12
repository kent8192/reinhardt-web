#[reinhardt::routes]
#[reinhardt::url_patterns]
pub fn root_urls() -> reinhardt::urls::prelude::UnifiedRouter {
	reinhardt::urls::prelude::UnifiedRouter::new()
		.server(|server| server.endpoint(crate::native_handlers::health))
		.mount_unified("/app/", crate::url_patterns())
		.merge(crate::another_app())
		.merge(crate::client_pages())
}
