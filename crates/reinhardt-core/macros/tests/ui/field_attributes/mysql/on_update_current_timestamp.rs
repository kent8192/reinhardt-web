#[cfg(feature = "db-mysql")]
use chrono::{DateTime, Utc};
#[cfg(feature = "db-mysql")]
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[cfg(feature = "db-mysql")]
#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "posts")]
struct Post {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 200)]
	title: String,

	#[field(auto_now_add = true)]
	created_at: DateTime<Utc>,

	#[field(on_update_current_timestamp = true)]
	updated_at: DateTime<Utc>,
}

fn main() {}
