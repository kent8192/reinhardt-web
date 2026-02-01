#[cfg(feature = "db-sqlite")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-sqlite")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true, autoincrement = true)]
	id: Option<i64>,

	#[field(max_length = 100)]
	username: String,
}

fn main() {}
