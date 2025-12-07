//! URL configuration for auth app (RESTful)

use crate::apps::auth::views;
use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new().endpoint(views::register)
}
