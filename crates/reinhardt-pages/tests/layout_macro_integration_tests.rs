#![cfg(not(target_arch = "wasm32"))]

use reinhardt_core::reactive::ReactiveScope;
use reinhardt_pages::{Outlet, Page, Path, component, layout, page};
use reinhardt_urls::routers::ClientRouter;

#[layout("/workspaces/{workspace_id}/", name = "workspace-shell")]
fn workspace_shell(Path(workspace_id): Path<i64>, outlet: Outlet) -> Page {
	page!(|workspace_id: i64, outlet: Outlet| {
		section {
			id: "workspace-shell",
			h1 { { format!("Workspace {workspace_id}") } }
			{ outlet }
		}
	})(workspace_id, outlet)
}

#[component("jobs", name = "workspace-jobs")]
fn workspace_jobs(Path(workspace_id): Path<i64>) -> Page {
	page!(|workspace_id: i64| {
		p {
			id: "workspace-jobs",
			{ format!("Jobs for {workspace_id}") }
		}
	})(workspace_id)
}

#[test]
fn layout_macro_registers_route_tree_shell_and_child() {
	ReactiveScope::run(|| {
		let router = ClientRouter::new().routes(|routes| {
			routes.layout(workspace_shell, |children| {
				children.component(workspace_jobs)
			})
		});

		let html = router.render_path("/workspaces/42/jobs").render_to_string();
		assert!(
			html.contains("Workspace 42"),
			"layout should receive path params, got: {html}"
		);
		assert!(
			html.contains("Jobs for 42"),
			"child should render inside layout outlet, got: {html}"
		);
		assert_eq!(
			router
				.reverse("workspace-jobs", &[("workspace_id", "42")])
				.expect("reverse child route"),
			"/workspaces/42/jobs"
		);
	});
}
