//! Unsupported backend should fail

use reinhardt_db_macros::document;

#[document(collection = "users", backend = "redis")]
struct User {
	id: String,
}

fn main() {}
