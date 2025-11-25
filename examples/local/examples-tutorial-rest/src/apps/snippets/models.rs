use chrono::{DateTime, Utc};
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

/// Snippet model representing a code snippet
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "snippets", table_name = "snippets")]
pub struct Snippet {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 100)]
	pub title: String,

	#[field(max_length = 10000)]
	pub code: String,

	#[field(max_length = 50)]
	pub language: String,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}

impl Snippet {
	/// Get a highlighted version of the code
	pub fn highlighted(&self) -> String {
		todo!("Implement syntax highlighting - use syntect or tree-sitter crate");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// TODO: Add database integration tests using TestContainers
	// Example:
	// #[rstest]
	// #[tokio::test]
	// async fn test_snippet_crud_operations(
	//     #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
	// ) {
	//     let (_container, db) = postgres_container.await;
	//     let manager = Manager::<Snippet>::new();
	//
	//     // Test create
	//     let snippet = Snippet {
	//         id: 0,
	//         title: "Hello World".to_string(),
	//         code: "println!(\"Hello, world!\");".to_string(),
	//         language: "rust".to_string(),
	//         created_at: Utc::now(),
	//     };
	//     let created = manager.create(snippet).await.unwrap();
	//
	//     // Test read
	//     assert_eq!(created.title, "Hello World");
	//
	//     // Test update, delete, etc.
	// }
}
