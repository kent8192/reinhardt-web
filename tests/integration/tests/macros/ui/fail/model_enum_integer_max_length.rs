#![allow(unexpected_cfgs)]

use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "i32")]
enum Priority {
	#[model_enum(value = 10)]
	Low,
	#[model_enum(value = 20)]
	High,
}

#[model(app_label = "jobs", table_name = "jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 4)]
	priority: Priority,
}

fn main() {}
