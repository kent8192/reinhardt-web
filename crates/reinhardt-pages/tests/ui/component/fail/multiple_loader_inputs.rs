use reinhardt_pages::{Loader, Page, Path, component, loader, page};

#[loader]
async fn project_loader(Path(_id): Path<i64>) -> Result<String, String> {
	Ok(String::new())
}

#[component("/projects/{id}/", name = "project", loader = project_loader)]
fn project(
	Path(id): Path<i64>,
	Loader(first): Loader<String>,
	Loader(second): Loader<String>,
) -> Page {
	page!(|| { p { { format!("{id}:{first}:{second}") } } })()
}

fn main() {}
