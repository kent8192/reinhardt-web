#![allow(unexpected_cfgs)]

use reinhardt::db::orm::expressions::FieldRef;
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
	#[field(max_length = 16)]
	status: Status,
	priority: Option<Priority>,
}

fn main() {
	let _: FieldRef<Job, Status> = Job::field_status();
	let _: FieldRef<Job, Option<Priority>> = Job::field_priority();
}
