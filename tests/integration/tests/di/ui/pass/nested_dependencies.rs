use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Clone)]
struct DatabaseService {
	connection_string: String,
}

#[async_trait::async_trait]
impl Injectable for DatabaseService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			connection_string: "postgres://localhost".to_string(),
		})
	}
}

#[derive(Clone)]
struct UserService {
	db: Arc<DatabaseService>,
}

#[async_trait::async_trait]
impl Injectable for UserService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let db = DatabaseService::inject(ctx).await?;
		Ok(Self { db: Arc::new(db) })
	}
}

#[tokio::main]
async fn main() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let user_service = UserService::inject(&ctx).await.unwrap();
	assert_eq!(user_service.db.connection_string, "postgres://localhost");
}
