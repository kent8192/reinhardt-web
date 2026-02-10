#[cfg(feature = "db-mysql")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-mysql")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "codes")]
struct Code {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(zerofill = true)]
	code: i32,
}

fn main() {}
