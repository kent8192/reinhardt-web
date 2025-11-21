use async_graphql::{Context, EmptySubscription, ID, Object, Result as GqlResult, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum GraphQLError {
	#[error("Schema error: {0}")]
	Schema(String),
	#[error("Resolver error: {0}")]
	Resolver(String),
	#[error("Not found: {0}")]
	NotFound(String),
}

pub type GraphQLResult<T> = Result<T, GraphQLError>;

/// Example: User type for GraphQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
	pub id: ID,
	pub name: String,
	pub email: String,
	pub active: bool,
}

#[Object]
impl User {
	async fn id(&self) -> &ID {
		&self.id
	}

	async fn name(&self) -> &str {
		&self.name
	}

	async fn email(&self) -> &str {
		&self.email
	}

	async fn active(&self) -> bool {
		self.active
	}
}

/// User storage (in-memory for example)
#[derive(Clone)]
pub struct UserStorage {
	users: Arc<RwLock<HashMap<String, User>>>,
}

impl UserStorage {
	/// Create a new user storage
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_graphql_core::schema::UserStorage;
	///
	/// let storage = UserStorage::new();
	// Creates a new storage instance with defaults
	/// ```
	pub fn new() -> Self {
		Self {
			users: Arc::new(RwLock::new(HashMap::new())),
		}
	}
	/// Add a user to storage
	///
	pub async fn add_user(&self, user: User) {
		self.users.write().await.insert(user.id.to_string(), user);
	}
	/// Get a user by ID
	///
	/// # Examples
	///
	/// ```ignore
	// Retrieve user
	/// let user = storage.get_user("user-1").await;
	/// ```
	pub async fn get_user(&self, id: &str) -> Option<User> {
		self.users.read().await.get(id).cloned()
	}
	/// List all users
	///
	/// # Examples
	///
	/// ```ignore
	// List all users
	/// let users = storage.list_users().await;
	/// ```
	pub async fn list_users(&self) -> Vec<User> {
		self.users.read().await.values().cloned().collect()
	}
}

impl Default for UserStorage {
	fn default() -> Self {
		Self::new()
	}
}

/// GraphQL Query root
pub struct Query;

#[Object]
impl Query {
	async fn user(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<User>> {
		let storage = ctx.data::<UserStorage>()?;
		Ok(storage.get_user(id.as_ref()).await)
	}

	async fn users(&self, ctx: &Context<'_>) -> GqlResult<Vec<User>> {
		let storage = ctx.data::<UserStorage>()?;
		Ok(storage.list_users().await)
	}

	async fn hello(&self, name: Option<String>) -> String {
		format!("Hello, {}!", name.unwrap_or_else(|| "World".to_string()))
	}
}

/// Input type for creating users
#[derive(async_graphql::InputObject)]
pub struct CreateUserInput {
	pub name: String,
	pub email: String,
}

/// GraphQL Mutation root
pub struct Mutation;

#[Object]
impl Mutation {
	async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> GqlResult<User> {
		let storage = ctx.data::<UserStorage>()?;

		let user = User {
			id: ID::from(uuid::Uuid::new_v4().to_string()),
			name: input.name,
			email: input.email,
			active: true,
		};

		storage.add_user(user.clone()).await;
		Ok(user)
	}

	async fn update_user_status(
		&self,
		ctx: &Context<'_>,
		id: ID,
		active: bool,
	) -> GqlResult<Option<User>> {
		let storage = ctx.data::<UserStorage>()?;

		if let Some(mut user) = storage.get_user(id.as_ref()).await {
			user.active = active;
			storage.add_user(user.clone()).await;
			Ok(Some(user))
		} else {
			Ok(None)
		}
	}
}

/// Create GraphQL schema
pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;
/// Documentation for `create_schema`
///
pub fn create_schema(storage: UserStorage) -> AppSchema {
	Schema::build(Query, Mutation, EmptySubscription)
		.data(storage)
		.finish()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_query_hello() {
		let storage = UserStorage::new();
		let schema = create_schema(storage);

		let query = r#"
            {
                hello(name: "GraphQL")
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert_eq!(data["hello"], "Hello, GraphQL!");
	}

	#[tokio::test]
	async fn test_mutation_create_user() {
		let storage = UserStorage::new();
		let schema = create_schema(storage);

		let query = r#"
            mutation {
                createUser(input: { name: "Alice", email: "alice@example.com" }) {
                    name
                    email
                    active
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert_eq!(data["createUser"]["name"], "Alice");
		assert_eq!(data["createUser"]["active"], true);
	}

	#[tokio::test]
	async fn test_query_user() {
		let storage = UserStorage::new();
		let user = User {
			id: ID::from("test-id-123"),
			name: "Bob".to_string(),
			email: "bob@example.com".to_string(),
			active: true,
		};
		storage.add_user(user).await;

		let schema = create_schema(storage);

		let query = r#"
            {
                user(id: "test-id-123") {
                    id
                    name
                    email
                    active
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert_eq!(data["user"]["id"], "test-id-123");
		assert_eq!(data["user"]["name"], "Bob");
		assert_eq!(data["user"]["email"], "bob@example.com");
		assert_eq!(data["user"]["active"], true);
	}

	#[tokio::test]
	async fn test_query_user_not_found() {
		let storage = UserStorage::new();
		let schema = create_schema(storage);

		let query = r#"
            {
                user(id: "nonexistent-id") {
                    id
                    name
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert!(data["user"].is_null());
	}

	#[tokio::test]
	async fn test_query_users_empty() {
		let storage = UserStorage::new();
		let schema = create_schema(storage);

		let query = r#"
            {
                users {
                    id
                    name
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert!(data["users"].is_array());
		assert_eq!(data["users"].as_array().unwrap().len(), 0);
	}

	#[tokio::test]
	async fn test_query_users_multiple() {
		let storage = UserStorage::new();

		let user1 = User {
			id: ID::from("1"),
			name: "Alice".to_string(),
			email: "alice@example.com".to_string(),
			active: true,
		};
		let user2 = User {
			id: ID::from("2"),
			name: "Bob".to_string(),
			email: "bob@example.com".to_string(),
			active: false,
		};
		let user3 = User {
			id: ID::from("3"),
			name: "Charlie".to_string(),
			email: "charlie@example.com".to_string(),
			active: true,
		};

		storage.add_user(user1).await;
		storage.add_user(user2).await;
		storage.add_user(user3).await;

		let schema = create_schema(storage);

		let query = r#"
            {
                users {
                    id
                    name
                    email
                    active
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		let users = data["users"].as_array().unwrap();
		assert_eq!(users.len(), 3);

		// Verify that all users are present
		let names: Vec<&str> = users.iter().map(|u| u["name"].as_str().unwrap()).collect();
		assert!(names.contains(&"Alice"));
		assert!(names.contains(&"Bob"));
		assert!(names.contains(&"Charlie"));
	}

	#[tokio::test]
	async fn test_mutation_update_user_status() {
		let storage = UserStorage::new();
		let user = User {
			id: ID::from("update-test-id"),
			name: "David".to_string(),
			email: "david@example.com".to_string(),
			active: true,
		};
		storage.add_user(user).await;

		let schema = create_schema(storage);

		let query = r#"
            mutation {
                updateUserStatus(id: "update-test-id", active: false) {
                    id
                    name
                    active
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert_eq!(data["updateUserStatus"]["id"], "update-test-id");
		assert_eq!(data["updateUserStatus"]["active"], false);
	}

	#[tokio::test]
	async fn test_mutation_update_nonexistent_user() {
		let storage = UserStorage::new();
		let schema = create_schema(storage);

		let query = r#"
            mutation {
                updateUserStatus(id: "does-not-exist", active: false) {
                    id
                    name
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert!(data["updateUserStatus"].is_null());
	}

	#[tokio::test]
	async fn test_user_object_fields() {
		let user = User {
			id: ID::from("field-test-id"),
			name: "Eve".to_string(),
			email: "eve@example.com".to_string(),
			active: false,
		};

		// Test direct field access
		assert_eq!(user.id.to_string(), "field-test-id");
		assert_eq!(user.name, "Eve");
		assert_eq!(user.email, "eve@example.com");
		assert_eq!(user.active, false);
	}

	#[tokio::test]
	async fn test_user_storage_add_get() {
		let storage = UserStorage::new();

		let user = User {
			id: ID::from("storage-test-1"),
			name: "Frank".to_string(),
			email: "frank@example.com".to_string(),
			active: true,
		};

		storage.add_user(user.clone()).await;

		let retrieved = storage.get_user("storage-test-1").await;
		let retrieved = retrieved.unwrap();
		assert_eq!(retrieved.id.to_string(), "storage-test-1");
		assert_eq!(retrieved.name, "Frank");
		assert_eq!(retrieved.email, "frank@example.com");
		assert_eq!(retrieved.active, true);
	}

	#[tokio::test]
	async fn test_user_storage_list() {
		let storage = UserStorage::new();

		// Initially empty
		let users = storage.list_users().await;
		assert_eq!(users.len(), 0);

		// Add users
		storage
			.add_user(User {
				id: ID::from("list-1"),
				name: "User1".to_string(),
				email: "user1@example.com".to_string(),
				active: true,
			})
			.await;

		storage
			.add_user(User {
				id: ID::from("list-2"),
				name: "User2".to_string(),
				email: "user2@example.com".to_string(),
				active: false,
			})
			.await;

		let users = storage.list_users().await;
		assert_eq!(users.len(), 2);
	}

	#[tokio::test]
	async fn test_create_schema_with_data() {
		let storage = UserStorage::new();
		storage
			.add_user(User {
				id: ID::from("pre-existing"),
				name: "PreExisting".to_string(),
				email: "preexisting@example.com".to_string(),
				active: true,
			})
			.await;

		let schema = create_schema(storage);

		// Verify schema can query pre-existing data
		let query = r#"
            {
                user(id: "pre-existing") {
                    name
                }
            }
        "#;

		let result = schema.execute(query).await;
		let data = result.data.into_json().unwrap();
		assert_eq!(data["user"]["name"], "PreExisting");
	}

	#[tokio::test]
	async fn test_graphql_error_types() {
		let err1 = GraphQLError::Schema("test schema error".to_string());
		assert!(err1.to_string().contains("Schema error"));

		let err2 = GraphQLError::Resolver("test resolver error".to_string());
		assert!(err2.to_string().contains("Resolver error"));

		let err3 = GraphQLError::NotFound("test item".to_string());
		assert!(err3.to_string().contains("Not found"));
	}
}
