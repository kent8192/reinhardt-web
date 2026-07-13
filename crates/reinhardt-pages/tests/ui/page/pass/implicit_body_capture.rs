//! UI compile-pass test for implicit captures in direct `page!({ ... })` form.

use reinhardt_pages::callback::Callback;
use reinhardt_pages::component::Page;
use reinhardt_pages::page;

#[derive(Clone)]
struct Todo {
	id: String,
	title: String,
	completed: bool,
}

#[derive(Clone)]
struct Label(String);

impl Label {
	fn text(&self) -> String {
		self.0.clone()
	}
}

#[derive(bon::Builder)]
struct PanelProps {
	title: String,
	on_click: Option<Callback<(), ()>>,
	children: Option<Page>,
}

fn panel(props: PanelProps) -> Page {
	page!(|props: PanelProps| {
		section {
			h2 { { props.title.clone() } }
			{ props.children.clone().unwrap_or_else(Page::empty) }
		}
	})(props)
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let heading = Label("Todos".to_string());
		let button_label = Label("Refresh".to_string());
		let selected = "todo-1".to_string();
		let todos = vec![
			Todo {
				id: "todo-1".to_string(),
				title: "Ship body form".to_string(),
				completed: false,
			},
			Todo {
				id: "todo-2".to_string(),
				title: "Document migration".to_string(),
				completed: true,
			},
		];

		let view: Page = page!({
			div {
				Panel {
					title: heading.text(),
					@click: Callback::new(|_: ()| {}),
					p { { selected.clone() } }
				}
				if !todos.is_empty() {
					ul {
						for todo in todos @key(format!("{}:{}", selected.as_str(), todo.id)) {
							li {
								class: if todo.completed { "done" } else { "pending" },
								{ todo.title.clone() }
							}
						}
					}
				}
				else {
					p { "No todos" }
				}button {
					@click: move |_| {
						let _ = button_label.text();
					},
					{ button_label.text() }
				}
			}
		});

		let _ = view;
	});
}
