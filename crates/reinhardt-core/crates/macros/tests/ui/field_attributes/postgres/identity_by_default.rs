#[cfg(feature = "db-postgres")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-postgres")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "posts")]
struct Post {
	#[field(primary_key = true, identity_by_default = true)]
	id: Option<i64>,

	#[field(max_length = 200)]
	title: String,
}

fn main() {}
