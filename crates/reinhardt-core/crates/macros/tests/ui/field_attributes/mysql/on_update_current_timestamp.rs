#[cfg(feature = "db-mysql")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-mysql")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "posts")]
struct Post {
	#[field(primary_key = true)]
	id: Option<i32>,

	title: String,

	#[field(auto_now_add = true)]
	created_at: NaiveDateTime,

	#[field(on_update_current_timestamp = true)]
	updated_at: NaiveDateTime,
}

fn main() {}
