#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
use reinhardt_macros::model;
#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
#[model(app_label = "test", table_name = "users")]
#[derive(Serialize, Deserialize)]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 255, comment = "User's email address")]
	email: String,
}

fn main() {}
