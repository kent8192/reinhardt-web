#[cfg(feature = "db-sqlite")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

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
