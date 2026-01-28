//! Document without collection attribute should fail

use reinhardt_db_macros::document;

#[document(backend = "mongodb")]
struct User {
	id: String,
}

fn main() {}
