#![deny(unexpected_cfgs)]

use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Database;

struct DatabaseKey;

impl reinhardt_di::InjectableKey for DatabaseKey {}

#[derive(Serialize, Deserialize)]
struct User {
	id: u32,
}

#[server_fn]
async fn get_user(
	id: u32,
	#[inject] _database: reinhardt_di::Depends<DatabaseKey, Database>,
) -> Result<User, reinhardt_pages::server_fn::ServerFnError> {
	Ok(User { id })
}

fn main() {
	let _ = get_user;
}
