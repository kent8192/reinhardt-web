//! Browser-level coverage for route-loader prepare/commit navigation.
//!
//! These tests exercise the pages-owned coordinator through the public
//! `RouterHandle` API. A destination is not rendered until every matched
//! loader has prepared successfully; failures leave both the old URL and the
//! old DOM mounted.

#![cfg(wasm)]

use reinhardt_pages::app::ClientLauncher;
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::reactive::hooks::RouterHandle;
use reinhardt_pages::{Loader, component, loader};
use reinhardt_urls::routers::ClientRouter;
use std::cell::Cell;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
	static LOADED_CALLS: Cell<u32> = const { Cell::new(0) };
}

fn home_page() -> Page {
	PageElement::new("div")
		.attr("id", "route-home")
		.child("HOME")
		.into_page()
}

#[loader]
async fn loaded_loader() -> Result<String, String> {
	LOADED_CALLS.with(|calls| calls.set(calls.get() + 1));
	Ok("LOADED DATA".to_string())
}

#[component("/loaded", name = "loader-navigation-loaded", loader = loaded_loader)]
fn loaded_page(Loader(data): Loader<String>) -> Page {
	PageElement::new("div")
		.attr("id", "route-loaded")
		.child(data)
		.into_page()
}

#[loader]
async fn slow_loader() -> Result<String, String> {
	gloo_timers::future::TimeoutFuture::new(30).await;
	Ok("SLOW DATA".to_string())
}

#[component("/slow", name = "loader-navigation-slow", loader = slow_loader)]
fn slow_page(Loader(data): Loader<String>) -> Page {
	PageElement::new("div")
		.attr("id", "route-slow")
		.child(data)
		.into_page()
}

#[loader]
async fn failing_loader() -> Result<String, String> {
	Err("safe loader failure".to_string())
}

#[component("/failed", name = "loader-navigation-failed", loader = failing_loader)]
fn failed_page(Loader(data): Loader<String>) -> Page {
	PageElement::new("div")
		.attr("id", "route-failed")
		.child(data)
		.into_page()
}

fn install_app_root() -> web_sys::Element {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let history = web_sys::window()
		.expect("window")
		.history()
		.expect("history");
	history
		.replace_state_with_url(&JsValue::NULL, "", Some("/"))
		.expect("reset history path");
	if let Some(previous) = document.get_element_by_id("app") {
		previous.remove();
	}
	let root = document.create_element("div").expect("create root");
	root.set_id("app");
	document
		.body()
		.expect("body")
		.append_child(&root)
		.expect("append root");
	root
}

fn build_router() -> ClientRouter {
	ClientRouter::new()
		.route("home", "/", home_page)
		.component(loaded_page)
		.component(slow_page)
		.component(failed_page)
}

async fn yield_to_tasks() {
	gloo_timers::future::TimeoutFuture::new(0).await;
}

fn current_path() -> String {
	web_sys::window()
		.expect("window")
		.location()
		.pathname()
		.expect("pathname")
}

#[wasm_bindgen_test]
async fn route_loader_navigation_commits_after_prepare() {
	let root = install_app_root();
	LOADED_CALLS.with(|calls| calls.set(0));
	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	assert!(root.inner_html().contains("HOME"));
	assert_eq!(current_path(), "/");
	let initial_state = web_sys::window()
		.expect("window")
		.history()
		.expect("history")
		.state()
		.expect("history state");
	assert_eq!(
		js_sys::Reflect::get(&initial_state, &JsValue::from_str("entry_index"))
			.expect("entry_index")
			.as_f64(),
		Some(0.0)
	);
	RouterHandle
		.push("/loaded")
		.expect("start loader navigation");

	// Matching and preparation are separate from commit: the old route remains
	// visible until the loader future settles.
	assert_eq!(current_path(), "/");
	assert!(root.inner_html().contains("HOME"));
	assert!(!root.inner_html().contains("LOADED DATA"));

	yield_to_tasks().await;
	yield_to_tasks().await;
	assert_eq!(current_path(), "/loaded");
	assert!(root.inner_html().contains("LOADED DATA"));
	let committed_state = web_sys::window()
		.expect("window")
		.history()
		.expect("history")
		.state()
		.expect("history state");
	assert_eq!(
		js_sys::Reflect::get(&committed_state, &JsValue::from_str("entry_index"))
			.expect("entry_index")
			.as_f64(),
		Some(1.0)
	);
	assert_eq!(LOADED_CALLS.with(Cell::get), 1);
}

#[wasm_bindgen_test]
async fn route_loader_navigation_retains_old_route_while_pending() {
	let root = install_app_root();
	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	RouterHandle.push("/slow").expect("start slow navigation");
	yield_to_tasks().await;
	assert_eq!(current_path(), "/");
	assert!(root.inner_html().contains("HOME"));
	assert!(!root.inner_html().contains("SLOW DATA"));

	gloo_timers::future::TimeoutFuture::new(45).await;
	yield_to_tasks().await;
	assert_eq!(current_path(), "/slow");
	assert!(root.inner_html().contains("SLOW DATA"));
}

#[wasm_bindgen_test]
async fn route_loader_navigation_failure_retains_old_route() {
	let root = install_app_root();
	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	RouterHandle
		.push("/failed")
		.expect("start failing navigation");
	yield_to_tasks().await;
	yield_to_tasks().await;
	assert_eq!(current_path(), "/");
	assert!(root.inner_html().contains("HOME"));
	assert!(!root.inner_html().contains("route-failed"));
}
