//! URL configuration for the Todo example.

use reinhardt::UnifiedRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;
use reinhardt::routes;

use crate::server_fn::{create_todo, delete_todo, list_todos, set_todo_completed};

/// Registers the Todo server functions for `manage runserver`.
#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().server(|s| {
		s.server_fn(list_todos::marker)
			.server_fn(create_todo::marker)
			.server_fn(set_todo_completed::marker)
			.server_fn(delete_todo::marker)
	})
}
