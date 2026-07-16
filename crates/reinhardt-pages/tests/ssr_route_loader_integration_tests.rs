#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::router::loader::{loader_cache_id, route_context};
use reinhardt_pages::router::loader_registry::LoaderRegistry;
use reinhardt_pages::{
	Loader, Outlet, Page, Path, RouteLoader, SsrRenderer, component, layout, loader, page,
};
use reinhardt_urls::routers::ClientRouter;
use std::time::Duration;

#[loader]
async fn ssr_greeting_loader() -> Result<String, String> {
	Ok("prepared on server".to_owned())
}

#[component(
	"/greeting/",
	name = "ssr-greeting",
	loader = ssr_greeting_loader
)]
fn ssr_greeting(Loader(message): Loader<String>) -> Page {
	page!(|message: String| {
		p { { message } }
	})(message)
}

#[loader]
async fn ssr_timeout_loader() -> Result<String, String> {
	tokio::time::sleep(Duration::from_millis(20)).await;
	Ok("too late".to_owned())
}

#[component("/timeout/", name = "ssr-timeout", loader = ssr_timeout_loader)]
fn ssr_timeout(Loader(message): Loader<String>) -> Page {
	page!(|message: String| {
		p { { message } }
	})(message)
}

#[loader]
async fn ssr_shell_loader(Path(workspace_id): Path<i64>) -> Result<String, String> {
	Ok(format!("shell-{workspace_id}"))
}

#[layout(
	"/ssr-workspaces/{workspace_id}/",
	name = "ssr-workspace-shell",
	loader = ssr_shell_loader,
)]
fn ssr_workspace_shell(
	Path(workspace_id): Path<i64>,
	Loader(data): Loader<String>,
	outlet: Outlet,
) -> Page {
	page!(|workspace_id: i64, data: String, outlet: Outlet| {
		section {
			id: "ssr-shell",
			{ format!("SHELL {workspace_id} {data}") }
			{ outlet }
		}
	})(workspace_id, data, outlet)
}

#[loader]
async fn ssr_jobs_loader(Path(workspace_id): Path<i64>) -> Result<String, String> {
	Ok(format!("jobs-{workspace_id}"))
}

#[component("jobs", name = "ssr-workspace-jobs", loader = ssr_jobs_loader)]
fn ssr_workspace_jobs(Path(workspace_id): Path<i64>, Loader(data): Loader<String>) -> Page {
	page!(|workspace_id: i64, data: String| {
		p {
			id: "ssr-jobs",
			{ format!("JOBS {workspace_id} {data}") }
		}
	})(workspace_id, data)
}

#[test]
fn route_loader_is_prepared_before_ssr_render() {
	tokio_test::block_on(async {
		let router = ClientRouter::new().component(ssr_greeting);
		let mut renderer = SsrRenderer::new();

		let output = renderer.render_route_to_string(&router, "/greeting/").await;

		assert_eq!(output.status, 200);
		assert!(output.html.contains("prepared on server"));
		let loader_id = <ssr_greeting_loader::marker as RouteLoader>::ID;
		assert_eq!(
			renderer.state().get_route_loader_state(loader_id.as_str()),
			Some(&serde_json::json!("prepared on server"))
		);
		let matched = router.match_tree("/greeting/").expect("route matches");
		let key = loader_cache_id(loader_id, &route_context(&matched), &[])
			.expect("loader key is deterministic");
		assert_eq!(
			renderer.state().get_resource_state(&key),
			Some(&serde_json::json!({ "Success": "prepared on server" }))
		);
		let registry = LoaderRegistry::global().expect("loader registry is available");
		registry
			.hydrate(
				loader_id,
				renderer
					.state()
					.get_route_loader_state(loader_id.as_str())
					.expect("route-loader state is present"),
			)
			.expect("loader value deserializes for hydration");
	});
}

#[test]
fn route_loader_timeout_returns_safe_status() {
	tokio_test::block_on(async {
		let router = ClientRouter::new().component(ssr_timeout);
		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::SsrOptions::new().resource_timeout(Duration::from_millis(1)),
		);

		let output = renderer.render_route_to_string(&router, "/timeout/").await;

		assert_eq!(output.status, 504);
		assert!(output.html.contains("route loader timed out"));
		assert_eq!(renderer.state().resource_count(), 0);
	});
}

#[test]
fn nested_layout_and_leaf_loaders_prepare_in_parallel_for_ssr() {
	tokio_test::block_on(async {
		let router = ClientRouter::new().routes(|routes| {
			routes.layout(ssr_workspace_shell, |children| {
				children.component(ssr_workspace_jobs)
			})
		});
		let mut renderer = SsrRenderer::new();

		let output = renderer
			.render_route_to_string(&router, "/ssr-workspaces/7/jobs")
			.await;

		assert_eq!(output.status, 200);
		assert!(output.html.contains("SHELL 7 shell-7"));
		assert!(output.html.contains("JOBS 7 jobs-7"));
		let shell_id = <ssr_shell_loader::marker as RouteLoader>::ID;
		let jobs_id = <ssr_jobs_loader::marker as RouteLoader>::ID;
		assert_eq!(
			renderer.state().get_route_loader_state(shell_id.as_str()),
			Some(&serde_json::json!("shell-7"))
		);
		assert_eq!(
			renderer.state().get_route_loader_state(jobs_id.as_str()),
			Some(&serde_json::json!("jobs-7"))
		);
	});
}
