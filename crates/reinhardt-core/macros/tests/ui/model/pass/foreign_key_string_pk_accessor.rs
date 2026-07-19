use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(app_label = "default", table_name = "projects")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Project {
	#[field(primary_key = true, db_column = "project_key", max_length = 64)]
	id: Option<String>,
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
}

fn assert_string(_: String) {}

fn main() {
	let job_id = String::from("project-alpha");
	assert_string(job_id);
	let _load_project = Job::project_accessor;
}
