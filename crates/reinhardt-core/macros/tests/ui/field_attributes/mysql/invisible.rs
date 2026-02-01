#[cfg(feature = "db-mysql")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-mysql")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	username: String,

	#[field(max_length = 500, invisible = true)]
	internal_metadata: String,
}

fn main() {}
