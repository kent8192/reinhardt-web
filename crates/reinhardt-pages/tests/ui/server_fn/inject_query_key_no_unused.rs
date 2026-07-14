#![deny(unused_variables)]

use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Database;

struct DatabaseKey;

impl reinhardt_di::InjectableKey for DatabaseKey {}

#[derive(Serialize, Deserialize)]
pub struct User {
	id: u32,
	name: String,
}

#[server_fn]
async fn get_user(
	id: u32,
	#[inject] _db: reinhardt_di::KeyedDepends<DatabaseKey, Database>,
) -> Result<User, reinhardt_pages::server_fn::ServerFnError> {
	Ok(User {
		id,
		name: format!("User {}", id),
	})
}

fn main() {
	let _ = get_user;
}
