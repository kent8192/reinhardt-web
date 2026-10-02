use reinhardt_pages::{Path, RouteLoader, RouteLoaderError, loader};

#[loader]
pub async fn project_loader(Path(project_id): Path<i64>) -> Result<String, RouteLoaderError> {
	Ok(project_id.to_string())
}

fn main() {
	let _future = project_loader(Path(42));
	let _: reinhardt_pages::RouteLoaderId = <project_loader::marker as RouteLoader>::ID;
}
