#![cfg(not(target_arch = "wasm32"))]

use examples_todo::server_fn::{
	clear_todos_for_test, create_todo, delete_todo, list_todos, set_todo_completed,
};
use examples_todo::todo::TodoFilter;
use examples_todo::ui::todo_page;
use serial_test::serial;

#[tokio::test]
#[serial(todo_store)]
async fn todo_server_functions_create_filter_update_and_delete() {
	clear_todos_for_test();

	let first = create_todo("Write spec".to_string())
		.await
		.expect("create_todo should create the first item");
	let second = create_todo("Ship example".to_string())
		.await
		.expect("create_todo should create the second item");

	assert_eq!(first.id, 1);
	assert_eq!(second.id, 2);
	assert_eq!(list_todos(TodoFilter::All).await.unwrap().len(), 2);
	assert_eq!(list_todos(TodoFilter::Active).await.unwrap().len(), 2);
	assert!(list_todos(TodoFilter::Completed).await.unwrap().is_empty());

	let completed = set_todo_completed(first.id, true)
		.await
		.expect("set_todo_completed should update the first item");
	assert!(completed.completed);
	assert_eq!(list_todos(TodoFilter::Active).await.unwrap(), vec![second]);
	assert_eq!(
		list_todos(TodoFilter::Completed).await.unwrap(),
		vec![completed]
	);

	delete_todo(first.id)
		.await
		.expect("delete_todo should remove the first item");
	assert_eq!(list_todos(TodoFilter::All).await.unwrap().len(), 1);
}

#[tokio::test]
#[serial(todo_store)]
async fn create_todo_rejects_blank_title() {
	clear_todos_for_test();

	let result = create_todo("   ".to_string()).await;

	assert!(result.is_err());
	assert!(list_todos(TodoFilter::All).await.unwrap().is_empty());
}

#[test]
fn todo_page_renders_route_filters_and_empty_state() {
	let html = todo_page(TodoFilter::All).render_to_string();

	assert!(html.contains("href=\"/active/\""));
	assert!(html.contains("href=\"/completed/\""));
	assert!(html.contains("No todos yet."));
}
