use reinhardt_pages::{Page, Path, layout, page};

#[layout("/workspaces/{workspace_id}/", name = "workspace-shell")]
fn workspace_shell(Path(workspace_id): Path<i64>) -> Page {
	page!(|workspace_id: i64| {
		div { { workspace_id.to_string() } }
	})(workspace_id)
}

fn main() {}
