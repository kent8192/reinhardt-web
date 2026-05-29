//! Server functions for the Todo example.

use crate::todo::{TodoFilter, TodoItem};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[cfg(native)]
use std::sync::{Mutex, OnceLock};

#[cfg(native)]
#[derive(Debug)]
struct TodoStore {
	next_id: u64,
	todos: Vec<TodoItem>,
}

#[cfg(native)]
impl Default for TodoStore {
	fn default() -> Self {
		Self {
			next_id: 1,
			todos: Vec::new(),
		}
	}
}

#[cfg(native)]
static TODO_STORE: OnceLock<Mutex<TodoStore>> = OnceLock::new();

#[cfg(native)]
fn todo_store() -> &'static Mutex<TodoStore> {
	TODO_STORE.get_or_init(|| Mutex::new(TodoStore::default()))
}

/// Clears the in-memory store used by tests.
#[cfg(native)]
#[doc(hidden)]
pub fn clear_todos_for_test() {
	let mut store = todo_store()
		.lock()
		.expect("todo store mutex should not be poisoned");
	*store = TodoStore::default();
}

/// Lists Todo items matching `filter`.
#[server_fn]
pub async fn list_todos(filter: TodoFilter) -> std::result::Result<Vec<TodoItem>, ServerFnError> {
	let store = todo_store()
		.lock()
		.map_err(|_| ServerFnError::application("Todo store is unavailable"))?;

	Ok(store
		.todos
		.iter()
		.filter(|todo| filter.matches(todo))
		.cloned()
		.collect())
}

/// Creates an incomplete Todo item.
#[server_fn]
pub async fn create_todo(title: String) -> std::result::Result<TodoItem, ServerFnError> {
	let title = title.trim();
	if title.is_empty() {
		return Err(ServerFnError::server(400, "Todo title is required"));
	}

	let mut store = todo_store()
		.lock()
		.map_err(|_| ServerFnError::application("Todo store is unavailable"))?;
	let todo = TodoItem::new(store.next_id, title);
	store.next_id += 1;
	store.todos.push(todo.clone());

	Ok(todo)
}

/// Updates a Todo item's completion state.
#[server_fn]
pub async fn set_todo_completed(
	id: u64,
	completed: bool,
) -> std::result::Result<TodoItem, ServerFnError> {
	let mut store = todo_store()
		.lock()
		.map_err(|_| ServerFnError::application("Todo store is unavailable"))?;
	let todo = store
		.todos
		.iter_mut()
		.find(|todo| todo.id == id)
		.ok_or_else(|| ServerFnError::server(404, "Todo item not found"))?;
	todo.completed = completed;

	Ok(todo.clone())
}

/// Deletes a Todo item.
#[server_fn]
pub async fn delete_todo(id: u64) -> std::result::Result<(), ServerFnError> {
	let mut store = todo_store()
		.lock()
		.map_err(|_| ServerFnError::application("Todo store is unavailable"))?;
	let before = store.todos.len();
	store.todos.retain(|todo| todo.id != id);

	if store.todos.len() == before {
		return Err(ServerFnError::server(404, "Todo item not found"));
	}

	Ok(())
}
