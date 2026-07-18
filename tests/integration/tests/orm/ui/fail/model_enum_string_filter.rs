#![allow(unexpected_cfgs)]
//! Fail case: a string literal is not a typed model-enum field value.

use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
enum Status {
	#[model_enum(value = "queued")]
	Queued,
}

#[model(app_label = "jobs", table_name = "jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 16)]
	status: Status,
}

fn main() {
	let _ = Job::field_status().eq("queued");
}
