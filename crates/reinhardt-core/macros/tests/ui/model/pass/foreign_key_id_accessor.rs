use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(app_label = "default", table_name = "projects")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Project {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 120)]
	name: String,
}

#[model(app_label = "default", table_name = "jobs")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: i64,
	#[rel(foreign_key)]
	project: db::associations::ForeignKeyField<Project>,
	#[field(max_length = 120)]
	job_type: String,
}

impl Job {
	fn retry_preserves_input(&self, retry: &Self) -> bool {
		self.project_id() == retry.project_id() && self.job_type == retry.job_type
	}
}

fn assert_i64(_: i64) {}

fn main() {
	let job = Job::build().project(7_i64).job_type("draft").finish();
	let retry = Job::build().project(7_i64).job_type("draft").finish();

	assert_i64(job.project_id());
	assert!(job.retry_preserves_input(&retry));
}
