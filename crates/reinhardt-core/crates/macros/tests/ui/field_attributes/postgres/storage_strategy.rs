#[cfg(feature = "db-postgres")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

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
