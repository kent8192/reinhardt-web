#[cfg(feature = "db-mysql")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-mysql")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "articles")]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(
		max_length = 200,
		character_set = "utf8mb4",
		collate = "utf8mb4_unicode_ci"
	)]
	title: String,
}

fn main() {}
