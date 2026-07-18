#![allow(unexpected_cfgs)]

use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
enum Status {
	#[model_enum(value = "eclair")]
	Ascii,
	#[model_enum(value = "éclair")]
	Unicode,
}

#[model(app_label = "jobs", table_name = "jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 5)]
	status: Status,
}

fn main() {}
