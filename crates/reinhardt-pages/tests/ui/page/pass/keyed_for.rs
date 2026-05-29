//! page! macro with keyed for loop rendering

use reinhardt_pages::page;

#[derive(Clone)]
struct Todo {
	id: String,
	title: String,
	completed: bool,
}

fn main() {
	let _keyed_for = page!(|todos: Vec<Todo>| {
		ul {
			for todo in todos @key(todo.id.clone()) {
				li {
					input {
						r#type: "checkbox",
						checked: todo.completed,
					}
					span { {
						todo.title.clone()
					} }
				}
			}
		}
	});
}
