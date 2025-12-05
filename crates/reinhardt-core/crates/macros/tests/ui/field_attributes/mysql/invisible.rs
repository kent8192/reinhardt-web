#[cfg(feature = "db-mysql")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-mysql")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	username: String,

	#[field(invisible = true)]
	internal_metadata: String,
}

fn main() {}
