mod loaders {
	use reinhardt_pages::{Path, loader};

	#[loader]
	pub async fn project_loader(Path(project_id): Path<i64>) -> Result<u64, String> {
		Ok(project_id as u64)
	}
}

use reinhardt_pages::{Loader, Page, Path, component, page};

#[component(
	"/projects/{project_id}/",
	name = "project",
	loader = loaders::project_loader,
)]
fn project(Path(project_id): Path<i64>, Loader(data): Loader<u64>) -> Page {
	page!(|project_id: i64, data: u64| {
		p { { format!("{project_id}:{data}") } }
	})(project_id, data)
}

fn main() {}
