#![allow(unexpected_cfgs)]
//! Pass case: typed field filters and assignments accept the field's Rust type.

use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
enum Status {
	#[model_enum(value = "queued")]
	Queued,
	#[model_enum(value = "running")]
	Running,
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
	let _ = Job::field_status().eq(Status::Queued);
	let _ = Job::field_status().is_in([Status::Queued, Status::Running]);
	let _ = Job::field_status().assign(Status::Running);
}
