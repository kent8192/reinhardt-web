#![cfg(feature = "client-router")]

use reinhardt_core::page::{IntoPage, Outlet, Page};
use reinhardt_urls::routers::client_router::{
	ClientRouter, FromRequest, LayoutInfo, PathParam, RouteContext, RouteRegistrationError,
};

#[derive(Debug)]
struct ShellProps {
	project_id: i64,
	outlet: Outlet,
}

impl reinhardt_urls::routers::client_router::FromLayoutRequest for ShellProps {
	fn from_layout_request(
		ctx: &RouteContext,
		outlet: Outlet,
	) -> Result<Self, reinhardt_urls::routers::client_router::ExtractError> {
		Ok(Self {
			project_id: PathParam::<i64>::extract(ctx, "project_id")?.into_inner(),
			outlet,
		})
	}
}

impl LayoutInfo for ShellProps {
	fn path() -> &'static str {
		"/writing/projects/{project_id}/"
	}

	fn name() -> &'static str {
		"writing-workspace"
	}

	fn component_name() -> &'static str {
		"WorkspaceShell"
	}

	fn function_name() -> &'static str {
		"workspace_shell"
	}

	fn props_type_name() -> &'static str {
		"ShellProps"
	}
}

#[derive(Debug)]
struct JobsProps {
	project_id: i64,
}

impl FromRequest for JobsProps {
	fn from_request(
		ctx: &RouteContext,
	) -> Result<Self, reinhardt_urls::routers::client_router::ExtractError> {
		Ok(Self {
			project_id: PathParam::<i64>::extract(ctx, "project_id")?.into_inner(),
		})
	}
}

impl reinhardt_urls::routers::client_router::ComponentInfo for JobsProps {
	fn path() -> &'static str {
		"jobs/"
	}

	fn name() -> &'static str {
		"writing-jobs"
	}

	fn component_name() -> &'static str {
		"Jobs"
	}

	fn function_name() -> &'static str {
		"jobs"
	}

	fn props_type_name() -> &'static str {
		"JobsProps"
	}
}

fn workspace_shell(props: ShellProps) -> Page {
	Page::fragment([
		Page::text(format!("shell {}", props.project_id)),
		props.outlet.into_page(),
	])
}

fn jobs(props: JobsProps) -> Page {
	Page::text(format!("jobs {}", props.project_id))
}

#[test]
fn layout_child_route_uses_composed_path_and_shared_params() {
	let router = ClientRouter::new()
		.try_routes(|routes| routes.layout(workspace_shell, |children| children.component(jobs)))
		.expect("route tree should register");

	let matched = router
		.match_tree("/writing/projects/7/jobs/")
		.expect("route should match");

	assert_eq!(matched.leaf().name(), Some("writing-jobs"));
	assert_eq!(matched.layouts().len(), 1);
	assert_eq!(
		matched.params().get("project_id").map(String::as_str),
		Some("7")
	);
	assert_eq!(
		router
			.reverse("writing-jobs", &[("project_id", "7")])
			.unwrap(),
		"/writing/projects/7/jobs/"
	);
	assert_eq!(
		router
			.render_path("/writing/projects/7/jobs/")
			.render_to_string(),
		"shell 7jobs 7"
	);
}

#[test]
fn layout_child_rejects_absolute_component_path() {
	#[derive(Debug)]
	struct BadProps;

	impl FromRequest for BadProps {
		fn from_request(
			_ctx: &RouteContext,
		) -> Result<Self, reinhardt_urls::routers::client_router::ExtractError> {
			Ok(Self)
		}
	}

	impl reinhardt_urls::routers::client_router::ComponentInfo for BadProps {
		fn path() -> &'static str {
			"/bad/"
		}

		fn name() -> &'static str {
			"bad"
		}

		fn component_name() -> &'static str {
			"Bad"
		}

		fn function_name() -> &'static str {
			"bad"
		}

		fn props_type_name() -> &'static str {
			"BadProps"
		}
	}

	fn bad(_props: BadProps) -> Page {
		Page::empty()
	}

	let err = ClientRouter::new()
		.try_routes(|routes| routes.layout(workspace_shell, |children| children.component(bad)))
		.unwrap_err();

	assert!(matches!(
		err,
		RouteRegistrationError::AbsolutePathInChildScope { .. }
	));
}

#[test]
fn index_route_matches_layout_base_path() {
	let router = ClientRouter::new()
		.try_routes(|routes| routes.layout(workspace_shell, |children| children.index(jobs)))
		.unwrap();

	assert_eq!(
		router
			.render_path("/writing/projects/9/")
			.render_to_string(),
		"shell 9jobs 9"
	);
}

#[test]
fn duplicate_param_names_across_layout_and_child_are_rejected() {
	#[derive(Debug)]
	struct BadChildProps;

	impl FromRequest for BadChildProps {
		fn from_request(
			_ctx: &RouteContext,
		) -> Result<Self, reinhardt_urls::routers::client_router::ExtractError> {
			Ok(Self)
		}
	}

	impl reinhardt_urls::routers::client_router::ComponentInfo for BadChildProps {
		fn path() -> &'static str {
			"dup/{project_id}/"
		}

		fn name() -> &'static str {
			"dup"
		}

		fn component_name() -> &'static str {
			"Dup"
		}

		fn function_name() -> &'static str {
			"dup"
		}

		fn props_type_name() -> &'static str {
			"BadChildProps"
		}
	}

	fn dup(_props: BadChildProps) -> Page {
		Page::empty()
	}

	let err = ClientRouter::new()
		.try_routes(|routes| routes.layout(workspace_shell, |children| children.component(dup)))
		.unwrap_err();

	assert!(matches!(
		err,
		RouteRegistrationError::DuplicatePathParam { name, .. } if name == "project_id"
	));
}
