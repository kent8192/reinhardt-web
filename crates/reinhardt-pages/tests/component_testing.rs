#![cfg(all(native, feature = "testing"))]

use reinhardt_core::types::page::{IntoPage, Page, PageElement};
use reinhardt_pages::page;
use reinhardt_pages::reactive::hooks::use_action;
use reinhardt_pages::reactive::{ResourceState, use_resource};
#[cfg(feature = "msw")]
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use reinhardt_pages::testing::component::{Role, render};

fn text_page(text: impl Into<String>) -> Page {
	PageElement::new("div").child(text.into()).into_page()
}

fn string_resource_page(state: ResourceState<String, String>) -> Page {
	match state {
		ResourceState::Loading => text_page("Loading"),
		ResourceState::Success(value) => text_page(value),
		ResourceState::Error(error) => text_page(error),
	}
}

fn index_job_component() -> Page {
	let resource = use_resource(
		|| async { Ok::<String, String>("Index job".to_string()) },
		(),
	);
	Page::reactive(move || string_resource_page(resource.get()))
}

fn ready_component() -> Page {
	let resource = use_resource(|| async { Ok::<String, String>("Ready".to_string()) }, ());
	Page::reactive(move || string_resource_page(resource.get()))
}

fn mixed_resource_component() -> Page {
	let pending = use_resource(|| std::future::pending::<Result<String, String>>(), ());
	let ready = use_resource(|| async { Ok::<String, String>("Ready".to_string()) }, ());
	Page::reactive(move || {
		let _ = pending.get();
		string_resource_page(ready.get())
	})
}

fn save_component() -> Page {
	let action = use_action(|_: ()| async { Ok::<String, String>("Saved".to_string()) });
	let button_action = action.clone();
	PageElement::new("div")
		.child(
			PageElement::new("button")
				.listener("click", move |_| button_action.dispatch(()))
				.child("Save"),
		)
		.child(Page::reactive(move || match action.result() {
			Some(value) => text_page(value),
			None => text_page("Idle"),
		}))
		.into_page()
}

#[test]
fn native_component_testing_public_surface_is_available() {
	let screen = render(
		PageElement::new("main")
			.attr("aria-label", "Dashboard")
			.child(PageElement::new("h1").child("Dashboard"))
			.child(PageElement::new("button").child("Refresh")),
	);

	assert_eq!(
		screen.get_by_role(Role::Main, "Dashboard").tag_name(),
		"main"
	);
	assert_eq!(
		screen.get_by_role(Role::Button, "Refresh").text(),
		"Refresh"
	);
}

#[tokio::test]
async fn settle_runs_use_resource_on_native() {
	let screen = render(index_job_component);

	screen.settle().await;

	assert!(screen.query_by_text("Index job").is_some());
}

#[tokio::test]
async fn click_action_settles_to_success() {
	let screen = render(save_component);

	screen.get_by_role(Role::Button, "Save").click();
	screen.settle().await;

	assert!(screen.query_by_text("Saved").is_some());
}

#[tokio::test]
async fn click_action_uses_own_screen_scheduler() {
	let first = render(save_component);
	let second = render(save_component);

	first.get_by_role(Role::Button, "Save").click();
	first.settle().await;

	assert!(first.query_by_text("Saved").is_some());
	assert!(second.query_by_text("Saved").is_none());
	assert!(second.query_by_text("Idle").is_some());
}

#[tokio::test]
async fn find_by_text_waits_for_resource() {
	let screen = render(ready_component);

	let element = screen.find_by_text("Ready").await;

	assert_eq!(element.text(), "Ready");
}

#[tokio::test]
async fn find_by_text_rerenders_completed_work_with_pending_tasks() {
	let screen = render(mixed_resource_component);

	let element = screen.find_by_text("Ready").await;

	assert_eq!(element.text(), "Ready");
}

#[test]
fn pretty_dom_snapshot_is_stable() {
	let screen = render(page!(|| {
		main {
			aria_label: "Jobs",
			button { "Refresh" }
		}
	}));

	insta::assert_snapshot!(screen.pretty());
}

#[cfg(feature = "msw")]
#[server_fn]
async fn load_jobs() -> Result<Vec<String>, ServerFnError> {
	Ok(vec!["real job".to_string()])
}

#[cfg(feature = "msw")]
fn jobs_resource_page(state: ResourceState<Vec<String>, ServerFnError>) -> Page {
	match state {
		ResourceState::Loading => text_page("Loading"),
		ResourceState::Success(values) => text_page(values.join(", ")),
		ResourceState::Error(error) => text_page(error.to_string()),
	}
}

#[cfg(feature = "msw")]
fn jobs_component() -> Page {
	let jobs = use_resource(|| async { load_jobs().await }, ());
	Page::reactive(move || jobs_resource_page(jobs.get()))
}

#[cfg(feature = "msw")]
#[tokio::test]
async fn server_fn_mock_feeds_resource() {
	let screen = render(jobs_component);
	screen.mock_server_fn::<load_jobs::marker>(|_args| Ok(vec!["Index job".to_string()]));

	screen.settle().await;

	assert!(screen.query_by_text("Index job").is_some());
	assert_eq!(screen.calls_to_server_fn::<load_jobs::marker>().len(), 1);
}

#[cfg(feature = "msw")]
#[tokio::test]
async fn server_fn_mocks_are_scoped_per_screen() {
	let first = render(jobs_component);
	first.mock_server_fn::<load_jobs::marker>(|_args| Ok(vec!["First job".to_string()]));
	let second = render(jobs_component);
	second.mock_server_fn::<load_jobs::marker>(|_args| Ok(vec!["Second job".to_string()]));

	first.settle().await;
	second.settle().await;

	assert!(first.query_by_text("First job").is_some());
	assert!(first.query_by_text("Second job").is_none());
	assert!(second.query_by_text("Second job").is_some());
	assert!(second.query_by_text("First job").is_none());
}

#[cfg(feature = "msw")]
#[tokio::test]
async fn server_fn_mock_errors_render_resource_errors() {
	let screen = render(jobs_component);
	screen.mock_server_fn::<load_jobs::marker>(|_args| {
		Err(ServerFnError::application("mock failed"))
	});

	screen.settle().await;

	assert!(
		screen
			.query_by_text("Application error: mock failed")
			.is_some()
	);
	assert_eq!(screen.calls_to_server_fn::<load_jobs::marker>().len(), 1);
}

#[cfg(feature = "msw")]
#[tokio::test]
async fn server_fn_mock_handler_can_inspect_recorded_calls() {
	let screen = render(jobs_component);
	let screen_for_handler = screen.clone();
	screen.mock_server_fn::<load_jobs::marker>(move |_args| {
		assert_eq!(
			screen_for_handler
				.calls_to_server_fn::<load_jobs::marker>()
				.len(),
			1
		);
		Ok(vec!["Inspectable job".to_string()])
	});

	screen.settle().await;

	assert!(screen.query_by_text("Inspectable job").is_some());
}
