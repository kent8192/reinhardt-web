//! Test that models without any primary key fail to compile

use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize)]
#[model(app_label = "test", table_name = "invalid_model")]
struct InvalidModel {
	#[field(max_length = 100)]
	name: String,

	age: i32,
}

fn main() {}
