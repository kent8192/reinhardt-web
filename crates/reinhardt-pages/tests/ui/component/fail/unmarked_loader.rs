use reinhardt_pages::{Loader, Page, Path, component, page};

async fn plain_loader(Path(_id): Path<i64>) -> Result<String, String> {
	Ok(String::new())
}

#[component("/projects/{id}/", name = "project", loader = plain_loader)]
fn project(Path(id): Path<i64>, Loader(data): Loader<String>) -> Page {
	page!(|id: i64, data: String| {
		p { { format!("{id}:{data}") } }
	})(id, data)
}

fn main() {}
