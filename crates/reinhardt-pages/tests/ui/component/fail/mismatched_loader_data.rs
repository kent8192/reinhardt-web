use reinhardt_pages::{Loader, Page, Path, component, loader, page};

#[loader]
async fn project_loader(Path(_id): Path<i64>) -> Result<u64, String> {
	Ok(1)
}

#[component("/projects/{id}/", name = "project", loader = project_loader)]
fn project(Path(id): Path<i64>, Loader(data): Loader<String>) -> Page {
	page!(|id: i64, data: String| {
		p { { format!("{id}:{data}") } }
	})(id, data)
}

fn main() {}
