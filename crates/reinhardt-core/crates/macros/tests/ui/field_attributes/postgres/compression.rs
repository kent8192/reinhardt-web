#[cfg(feature = "db-postgres")]
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[cfg(feature = "db-postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "logs")]
struct Log {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(compression = "lz4")]
	data: Vec<u8>,
}

fn main() {}
