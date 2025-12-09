//! URL configuration for auth app (RESTful)

use crate::apps::auth::views;
use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.with_namespace("auth")
		.endpoint(views::register)
		.endpoint(views::signin)
		.endpoint(views::signout)
		.endpoint(views::verify_password)
		.endpoint(views::change_password)
		.endpoint(views::reset_password)
		.endpoint(views::reset_password_confirm)
}
