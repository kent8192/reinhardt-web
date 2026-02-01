#[cfg(feature = "db-postgres")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-postgres")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "logs")]
struct Log {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 65535, compression = "lz4")]
	data: String,
}

fn main() {}
