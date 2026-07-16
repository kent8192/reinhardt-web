use reinhardt_pages::{Loader, Page, Path, component, loader, page};

#[loader]
async fn jobs_loader(Path(project_id): Path<i64>) -> Result<String, String> {
	Ok(project_id.to_string())
}

#[component(
	"/projects/{project_id}/jobs/",
	name = "jobs",
	loader = jobs_loader,
)]
fn jobs(Path(project_id): Path<i64>, Loader(data): Loader<String>) -> Page {
	page!(|project_id: i64, data: String| {
		p { { format!("{project_id}:{data}") } }
	})(project_id, data)
}

fn main() {
	let _props = JobsProps::builder().project_id(7).build();
	let _: Option<reinhardt_pages::RouteLoaderId> = <JobsProps as reinhardt_urls::routers::client_router::ComponentInfo>::loader_id();
}
