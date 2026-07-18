use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[derive(Default)]
struct ProjectManager;

#[model(table_name = "projects", manager = ProjectManager)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Project {
	#[field(primary_key = true, db_column = "project_pk")]
	id: i64,
	#[field(db_column = "project_code")]
	code: i64,
	#[field(max_length = 120)]
	name: String,
}

impl db::orm::CustomManager for ProjectManager {
	type Model = Project;

	fn filter(&self, _condition: db::orm::Filter) -> db::orm::Manager<Self::Model> {
		db::orm::Manager::default()
	}
}

#[model(table_name = "jobs")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
	#[field(primary_key = true)]
	id: i64,
	#[rel(
		foreign_key,
		db_column = "job_project_fk",
		to_field = "code"
	)]
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

struct TestExecutor;

impl db::orm::connection::OrmExecutor for TestExecutor {}

async fn load_project(job: &Job, executor: &mut TestExecutor) {
	let _ = job.project(executor).await;
}

fn main() {
	let job = Job::build().project(7_i64).job_type("draft").finish();
	let retry = Job::build().project(7_i64).job_type("draft").finish();

	assert_i64(job.project_id());
	assert!(job.retry_preserves_input(&retry));
	assert_eq!(<Project as db::orm::Model>::primary_key_column(), "project_pk");
	let _ = Job::project_accessor();
	let _ = load_project;
}
