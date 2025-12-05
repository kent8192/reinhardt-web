#[cfg(feature = "db-postgres")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "posts")]
struct Post {
	#[field(primary_key = true, identity_by_default = true)]
	id: Option<i64>,

	title: String,
}

fn main() {}
