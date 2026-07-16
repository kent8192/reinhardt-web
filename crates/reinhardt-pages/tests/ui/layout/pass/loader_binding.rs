use reinhardt_pages::{Loader, Outlet, Page, Path, layout, loader, page};

#[loader]
async fn shell_loader(Path(workspace_id): Path<i64>) -> Result<String, String> {
	Ok(workspace_id.to_string())
}

#[layout(
	"/workspaces/{workspace_id}/",
	name = "shell",
	loader = shell_loader,
)]
fn shell(
	Path(workspace_id): Path<i64>,
	Loader(data): Loader<String>,
	outlet: Outlet,
) -> Page {
	page!(|workspace_id: i64, data: String, outlet: Outlet| {
		section { { format!("{workspace_id}:{data}") } { outlet } }
	})(workspace_id, data, outlet)
}

fn main() {
	let _props = ShellProps::builder()
		.workspace_id(7)
		.outlet(Outlet::inline(Page::empty()))
		.build();
}
