#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
use reinhardt_macros::model;
#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
#[model(app_label = "test", table_name = "articles")]
#[derive(Serialize, Deserialize)]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 5000, fulltext = true)]
	content: String,
}

fn main() {}
