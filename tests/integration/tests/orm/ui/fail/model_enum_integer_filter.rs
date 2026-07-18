#![allow(unexpected_cfgs)]
//! Fail case: integer field inputs do not allow cross-width conversion.

use reinhardt::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "jobs", table_name = "jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: Option<i64>,
	retry_count: i32,
}

fn main() {
	let _ = Job::field_retry_count().eq(10_i64);
}
