#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
#[model(app_label = "test", table_name = "articles")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(fulltext = true)]
	content: String,
}

fn main() {}
