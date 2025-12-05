#[cfg(feature = "db-mysql")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-mysql")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "articles")]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(character_set = "utf8mb4", collate = "utf8mb4_unicode_ci")]
	title: String,
}

fn main() {}
