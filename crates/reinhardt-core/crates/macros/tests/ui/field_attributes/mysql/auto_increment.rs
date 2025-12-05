#[cfg(feature = "db-mysql")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-mysql")]
#[model(app_label = "test", table_name = "users")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	#[field(primary_key = true, auto_increment = true)]
	id: Option<u64>,

	username: String,
}

fn main() {}
