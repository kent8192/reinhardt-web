use reinhardt_pages::reactive::QueryKey;
use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

type AppResult<T> = Result<T, ServerFnError>;

#[derive(Serialize, Deserialize)]
pub struct User {
	id: u32,
	name: String,
}

#[server_fn]
async fn load_user(id: u32) -> AppResult<User> {
	Ok(User {
		id,
		name: format!("User {id}"),
	})
}

fn assert_user_key(_: QueryKey<User, ServerFnError>) {}

fn main() {
	assert_user_key(load_user::key(7));
}
