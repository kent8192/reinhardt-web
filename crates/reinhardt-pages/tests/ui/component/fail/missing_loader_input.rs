use reinhardt_pages::{Page, Path, component, loader, page};

#[loader]
async fn project_loader(Path(_id): Path<i64>) -> Result<String, String> {
	Ok(String::new())
}

#[component("/projects/{id}/", name = "project", loader = project_loader)]
fn project(Path(id): Path<i64>) -> Page {
	page!(|id: i64| {
		p { { id.to_string() } }
	})(id)
}

fn main() {}
