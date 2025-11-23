//! URL configuration for hello app

use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	let router = UnifiedRouter::new();

	// Add hello world endpoint
	// router.function("/", Method::GET, super::views::hello_world);

	router
}
