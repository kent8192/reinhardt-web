#[cfg(feature = "db-sqlite")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-sqlite")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true, autoincrement = true)]
	id: Option<i64>,

	username: String,
}

fn main() {}
