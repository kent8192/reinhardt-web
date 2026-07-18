use reinhardt_pages::{CancellationToken, Path, loader};

#[loader]
async fn cancellable_loader(
	Path(project_id): Path<i64>,
	CancellationToken(cancel): CancellationToken,
) -> Result<i64, String> {
	cancel.check().map_err(|error| error.to_string())?;
	Ok(project_id)
}

fn main() {
	let _loader = cancellable_loader;
}
