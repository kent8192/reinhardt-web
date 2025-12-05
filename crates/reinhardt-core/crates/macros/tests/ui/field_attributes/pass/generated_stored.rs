use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm as _;

#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	first_name: String,
	last_name: String,

	#[field(generated = "first_name || ' ' || last_name", generated_stored = true)]
	full_name: String,
}

fn main() {}
