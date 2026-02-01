#[cfg(feature = "db-postgres")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-postgres")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "documents")]
struct Document {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 10000, storage = "external")]
	large_text: String,
}

fn main() {}
