//! Todo pages UI built with `reinhardt-pages`.

use crate::server_fn::create_todo;
use crate::todo::{TodoFilter, TodoItem};
use reinhardt::ClientRouter;
use reinhardt::pages::Signal;
use reinhardt::pages::component::{IntoPage, Page, PageElement};
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action, use_state};

#[cfg(wasm)]
use {
	crate::server_fn::list_todos, reinhardt::pages::create_resource,
	reinhardt::pages::reactive::ResourceState, reinhardt::pages::reactive::hooks::use_effect,
	wasm_bindgen::JsCast,
};

/// Builds the client-side route table for the Todo SPA.
pub fn client_router() -> ClientRouter {
	ClientRouter::new()
		.route("todos:all", "/", || todo_page(TodoFilter::All))
		.route("todos:active", "/active/", || todo_page(TodoFilter::Active))
		.route("todos:completed", "/completed/", || {
			todo_page(TodoFilter::Completed)
		})
		.not_found(|| todo_page(TodoFilter::All))
}

/// Renders the Todo page for a route filter.
pub fn todo_page(filter: TodoFilter) -> Page {
	let (todos, _set_todos) = use_state(Vec::<TodoItem>::new());
	let (draft, _set_draft) = use_state(String::new());
	let (loading, _set_loading) = use_state(cfg!(wasm));
	let (error, _set_error) = use_state(None::<String>);

	let create_action = use_action(|title: String| async move {
		create_todo(title).await.map_err(|error| error.to_string())
	});

	#[cfg(wasm)]
	{
		let resource = create_resource(move || async move {
			list_todos(filter).await.map_err(|error| error.to_string())
		});
		let resource_for_effect = resource.clone();
		let resource_for_deps = resource.clone();
		let todos_setter = _set_todos.clone();
		let loading_setter = _set_loading.clone();
		let error_setter = _set_error.clone();

		use_effect(
			move || {
				match resource_for_effect.get() {
					ResourceState::Loading => {
						loading_setter(true);
						error_setter(None);
					}
					ResourceState::Success(items) => {
						todos_setter(items);
						loading_setter(false);
						error_setter(None);
					}
					ResourceState::Error(message) => {
						loading_setter(false);
						error_setter(Some(message));
					}
				}
				None::<fn()>
			},
			(resource_for_deps,),
		);
	}

	let todos_for_create = todos.clone();
	let draft_for_submit = draft.clone();
	let input_view = todo_input(draft.clone(), draft.clone());
	let current_todos = todos.get();
	let has_todos = !current_todos.is_empty();
	let todo_list = Page::Fragment(current_todos.into_iter().map(crate::ui::todo_row).collect());
	let is_loading = loading.get();
	let error_message = create_action.error().or_else(|| error.get());
	let has_error = error_message.is_some();
	let create_for_submit = create_action.clone();

	let content_view = if is_loading {
		status_line("todo-muted", "Loading todos...")
	} else if has_error {
		status_line("todo-error", error_message.unwrap_or_default()).attr("role", "alert")
	} else if !has_todos {
		status_line("todo-empty", crate::ui::empty_message(filter))
	} else {
		PageElement::new("ul")
			.attr("class", "todo-list")
			.child(todo_list)
	}
	.into_page();

	PageElement::new("main")
		.attr("class", "todo-shell")
		.child(
			PageElement::new("section")
				.attr("class", "todo-panel")
				.child(
					PageElement::new("header")
						.attr("class", "todo-header")
						.child(PageElement::new("h1").child("Todos"))
						.child(PageElement::new("p").child(
							"Signals, page! list rendering, server functions, and route filters in one small app.",
						)),
				)
				.child(
					PageElement::new("div")
						.attr("class", "todo-form")
						.child(
							PageElement::new("label")
								.attr("class", "sr-only")
								.attr("for", "new-todo")
								.child("New Todo"),
						)
						.child(input_view)
						.child(add_button(
							todos_for_create,
							draft_for_submit,
							create_for_submit,
						)),
				)
				.child(crate::ui::filter_nav(filter))
				.child(content_view),
		)
		.into_page()
}

fn add_button(
	todos: Signal<Vec<TodoItem>>,
	draft: Signal<String>,
	action: Action<TodoItem, String>,
) -> PageElement {
	let disabled = action.is_pending();
	PageElement::new("button")
		.attr("type", "button")
		.bool_attr("disabled", disabled)
		.listener("click", move |_event| {
			let title = draft.get().trim().to_string();
			if title.is_empty() {
				return;
			}
			let mut next = todos.get();
			let optimistic_id = next.iter().map(|todo| todo.id).max().unwrap_or(0) + 1;
			next.push(TodoItem::new(optimistic_id, title.clone()));
			todos.set(next);
			draft.set(String::new());
			action.dispatch(title);
		})
		.child("Add")
}

fn status_line(class: &'static str, message: impl IntoPage) -> PageElement {
	PageElement::new("p").attr("class", class).child(message)
}

#[cfg(wasm)]
pub(crate) fn todo_input(value: Signal<String>, draft_for_input: Signal<String>) -> Page {
	page!(|value: Signal<String>, draft_for_input: Signal<String>| {
		input {
			id: "new-todo",
			name: "title",
			placeholder: "Add a Todo",
			value: value.get(),
			@input: {
				let draft = draft_for_input.clone();
				move |event: web_sys::Event| {
					if let Some(target) = event.target() {
						if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
							draft.set(input.value());
						}
					}
				}
			},
		}
	})(value, draft_for_input)
}

#[cfg(native)]
pub(crate) fn todo_input(value: Signal<String>, _draft_for_input: Signal<String>) -> Page {
	page!(|value: Signal<String>| {
		input {
			id: "new-todo",
			name: "title",
			placeholder: "Add a Todo",
			value: value.get(),
			data_reactive: "true",
		}
	})(value)
}

pub(crate) fn filter_nav(current: TodoFilter) -> Page {
	page!(|current: TodoFilter| {
		nav {
			class: "todo-filters",
			aria_label: "Todo filters",
			{ crate::ui::filter_link(TodoFilter::All, current) }
			{ crate::ui::filter_link(TodoFilter::Active, current) }
			{ crate::ui::filter_link(TodoFilter::Completed, current) }
		}
	})(current)
}

pub(crate) fn filter_link(filter: TodoFilter, current: TodoFilter) -> Page {
	let class = if filter == current {
		"todo-filter is-active"
	} else {
		"todo-filter"
	}
	.to_string();
	page!(|filter: TodoFilter, class: String| {
		a {
			href: filter.path(),
			class: class,
			data_link: "true",
			{ filter.label() }
		}
	})(filter, class)
}

pub(crate) fn todo_row(todo: TodoItem) -> Page {
	let id = todo.id;
	let completed = todo.completed;
	let title = todo.title.clone();
	let row_class = if todo.completed {
		"todo-row is-completed"
	} else {
		"todo-row"
	}
	.to_string();
	page!(|id: u64, completed: bool, title: String, row_class: String| {
		li {
			class: row_class,
			label {
				class: "todo-checkline",
				input {
					type: "checkbox",
					checked: completed,
					@change: {
						let next_completed = !completed;
						move |_event| {
							reinhardt::pages::spawn::spawn_task(async move {
								let _ = crate::server_fn::set_todo_completed(id, next_completed).await;
							});
						}
					},
				}
				span { { title.clone() } }
			}
			button {
				class: "todo-delete",
				type: "button",
				aria_label: "Delete Todo",
				@click: {
					move |_event| {
						reinhardt::pages::spawn::spawn_task(async move {
							let _ = crate::server_fn::delete_todo(id).await;
						});
					}
				},
				"Remove"
			}
		}
	})(id, completed, title, row_class)
}

pub(crate) fn empty_message(filter: TodoFilter) -> &'static str {
	match filter {
		TodoFilter::All => "No todos yet.",
		TodoFilter::Active => "No active todos.",
		TodoFilter::Completed => "No completed todos.",
	}
}
