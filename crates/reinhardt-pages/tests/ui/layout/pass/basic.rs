use reinhardt_pages::{Outlet, Page, Path, component, layout, page};
use reinhardt_urls::routers::ClientRouter;

#[layout("/workspaces/{workspace_id}/", name = "workspace-shell")]
fn workspace_shell(Path(workspace_id): Path<i64>, outlet: Outlet) -> Page {
	page!(|workspace_id: i64, outlet: Outlet| {
		div {
			{ workspace_id.to_string() }
			{ outlet }
		}
	})(workspace_id, outlet)
}

#[component("jobs", name = "workspace-jobs")]
fn workspace_jobs(Path(workspace_id): Path<i64>) -> Page {
	page!(|workspace_id: i64| {
		p { { workspace_id.to_string() } }
	})(workspace_id)
}

fn main() {
	let _ = WorkspaceShellProps::builder()
		.workspace_id(7)
		.outlet(Outlet::inline(Page::empty()))
		.build();
	let _ = ClientRouter::new().routes(|routes| {
		routes.layout(workspace_shell, |children| {
			children.component(workspace_jobs)
		})
	});
}
