#![cfg(wasm)]

include!("../support/server_mutation_shared.rs");

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gloo_timers::future::TimeoutFuture;
use js_sys::{Function, Reflect};
use reinhardt_pages::MutationDispatchOutcome;
use reinhardt_pages::component::{PageExt, cleanup_reactive_nodes};
use reinhardt_pages::dom::Element;
use reinhardt_pages::hydration::hydrate;
use reinhardt_pages::prelude::defer_yield;
use reinhardt_pages::reactive::query::{QueryClient, QueryDefaults, QueryOptions};
use reinhardt_pages::reactive::{Effect, ReactiveScope, Signal};
use rstest::rstest;
use serial_test::serial;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

struct BodyRoot {
	element: web_sys::Element,
}

impl BodyRoot {
	fn new(id: &str) -> Self {
		let document = web_sys::window()
			.expect("browser window")
			.document()
			.expect("browser document");
		let element = document.create_element("div").expect("create root");
		element.set_id(id);
		document
			.body()
			.expect("browser body")
			.append_child(&element)
			.expect("append root");
		Self { element }
	}
}

impl Drop for BodyRoot {
	fn drop(&mut self) {
		cleanup_reactive_nodes();
		self.element.remove();
	}
}

struct SsrStateElement(web_sys::Element);

impl SsrStateElement {
	fn install() -> Self {
		let document = web_sys::window()
			.expect("browser window")
			.document()
			.expect("browser document");
		if let Some(existing) = document.get_element_by_id("ssr-state") {
			existing.remove();
		}
		let element = document.create_element("script").expect("create SSR state");
		element.set_id("ssr-state");
		element.set_text_content(Some("{}"));
		document
			.body()
			.expect("browser body")
			.append_child(&element)
			.expect("append SSR state");
		Self(element)
	}
}

impl Drop for SsrStateElement {
	fn drop(&mut self) {
		self.0.remove();
	}
}

struct FetchGuard {
	window: web_sys::Window,
	previous_fetch: JsValue,
}

impl FetchGuard {
	fn for_page_cluster() -> Self {
		let guard = Self::install();
		let global = js_sys::global();
		Reflect::set(
			global.as_ref(),
			&JsValue::from_str("__reinhardtPageClusterEndpoint"),
			&JsValue::from_str(&reinhardt_pages::server_fn::resolve_endpoint(
				"/api/server_fn/create_page_cluster",
			)),
		)
		.expect("install page endpoint");
		Reflect::set(
			global.as_ref(),
			&JsValue::from_str("__reinhardtPageClusterResolvers"),
			&js_sys::Array::new(),
		)
		.expect("install page resolvers");
		let stub = Function::new_no_args(
			r#"return function(request) {
    const expected = new URL(globalThis.__reinhardtPageClusterEndpoint,
                             window.location.href).pathname;
    if (new URL(request.url, window.location.href).pathname !== expected) {
        throw new Error(`unexpected page endpoint: ${request.url}`);
    }
    globalThis.__reinhardtCreateClusterRequests += 1;
    const ordinal = globalThis.__reinhardtCreateClusterRequests;
    return request.clone().text().then(body => {
        globalThis.__reinhardtCreateClusterBodies.push(body);
        return new Promise(resolve => {
            globalThis.__reinhardtPageClusterResolvers.push(() => {
                if (ordinal === 1) {
                    resolve(new Response(JSON.stringify({
                        version: 1,
                        kind: 'validation',
                        status: 422,
                        message: 'Validation failed',
                        field_errors: [
                            { field: 'name', message: 'Name is already used' },
                            { field: '_all', message: 'Cluster cannot be created' },
                            { field: 'org_id', message: 'Organization is unavailable' },
                            { field: 'unknown', message: 'Unknown field failure' }
                        ]
                    }), { status: 422 }));
                } else {
                    resolve(new Response(JSON.stringify({ token: 'one-time-token' }),
                                         { status: 200 }));
                }
            });
        });
    });
};"#,
		)
		.call0(&JsValue::NULL)
		.expect("build page fetch stub");
		Reflect::set(guard.window.as_ref(), &JsValue::from_str("fetch"), &stub)
			.expect("install page fetch stub");
		guard
	}

	fn release_page_requests(&self) {
		if let Ok(value) = Reflect::get(
			js_sys::global().as_ref(),
			&JsValue::from_str("__reinhardtPageClusterResolvers"),
		) && let Ok(resolvers) = value.dyn_into::<js_sys::Array>()
		{
			for resolver in resolvers.iter() {
				if let Some(resolver) = resolver.dyn_ref::<Function>() {
					let _ = resolver.call0(&JsValue::NULL);
				}
			}
			resolvers.set_length(0);
		}
	}

	fn install() -> Self {
		let window = web_sys::window().expect("browser window");
		let global = js_sys::global();
		let previous_fetch =
			Reflect::get(window.as_ref(), &JsValue::from_str("fetch")).expect("read window.fetch");
		for (key, value) in [
			(
				"__reinhardtCreateClusterEndpoint",
				JsValue::from_str(&reinhardt_pages::server_fn::resolve_endpoint(
					"/api/server_fn/create_cluster",
				)),
			),
			(
				"__reinhardtUpdateClusterEndpoint",
				JsValue::from_str(&reinhardt_pages::server_fn::resolve_endpoint(
					"/api/server_fn/update_cluster",
				)),
			),
			(
				"__reinhardtDeleteClusterEndpoint",
				JsValue::from_str(&reinhardt_pages::server_fn::resolve_endpoint(
					"/api/server_fn/delete_cluster",
				)),
			),
			("__reinhardtCreateClusterRequests", JsValue::from_f64(0.0)),
			("__reinhardtUpdateClusterRequests", JsValue::from_f64(0.0)),
			("__reinhardtDeleteClusterRequests", JsValue::from_f64(0.0)),
			("__reinhardtDeleteClusterGateOpen", JsValue::FALSE),
		] {
			Reflect::set(global.as_ref(), &JsValue::from_str(key), &value)
				.expect("install fetch fixture global");
		}
		Reflect::set(
			global.as_ref(),
			&JsValue::from_str("__reinhardtCreateClusterBodies"),
			&js_sys::Array::new(),
		)
		.expect("install create request body array");
		Reflect::set(
			global.as_ref(),
			&JsValue::from_str("__reinhardtDeleteClusterResolvers"),
			&js_sys::Array::new(),
		)
		.expect("install delete resolver array");
		let stub = Function::new_no_args(
			r#"
				return function(request) {
					const url = typeof request === 'string' ? request : request.url;
					const path = new URL(url, window.location.href).pathname;
					const validation = JSON.stringify({
						version: 1,
						kind: 'validation',
						status: 422,
						message: 'Validation failed',
						field_errors: [{ field: 'name', message: 'Name is already used' }],
					});
					const createEndpoint = new URL(
						globalThis.__reinhardtCreateClusterEndpoint,
						window.location.href,
					).pathname;
					const updateEndpoint = new URL(
						globalThis.__reinhardtUpdateClusterEndpoint,
						window.location.href,
					).pathname;
					const deleteEndpoint = new URL(
						globalThis.__reinhardtDeleteClusterEndpoint,
						window.location.href,
					).pathname;
					if (path === createEndpoint) {
						globalThis.__reinhardtCreateClusterRequests += 1;
						return request.clone().text().then((body) => {
							globalThis.__reinhardtCreateClusterBodies.push(body);
							if (globalThis.__reinhardtCreateClusterRequests === 1) {
								return new Response(validation, { status: 422 });
							}
							return new Response(
								JSON.stringify({ token: 'one-time-token' }),
								{ status: 200 }
							);
						});
					}
					if (path === updateEndpoint) {
						globalThis.__reinhardtUpdateClusterRequests += 1;
						if (globalThis.__reinhardtUpdateClusterRequests === 1) {
							return Promise.resolve(new Response(validation, { status: 422 }));
						}
						return Promise.resolve(
							new Response(
								JSON.stringify({ cluster_id: 'cluster-1', name: 'updated cluster' }),
								{ status: 200 }
							)
						);
					}
					if (path === deleteEndpoint) {
						globalThis.__reinhardtDeleteClusterRequests += 1;
						return new Promise((resolve) => {
							if (globalThis.__reinhardtDeleteClusterGateOpen) {
								resolve(new Response('null', { status: 200 }));
								return;
							}
							globalThis.__reinhardtDeleteClusterResolvers.push(resolve);
						});
					}
					throw new Error(`unexpected fetch endpoint: ${path}`);
				};
			"#,
		)
		.call0(&JsValue::NULL)
		.expect("build fetch stub");
		Reflect::set(window.as_ref(), &JsValue::from_str("fetch"), &stub)
			.expect("install fetch stub");

		Self {
			window,
			previous_fetch,
		}
	}

	fn counter(&self, key: &str) -> u32 {
		Reflect::get(js_sys::global().as_ref(), &JsValue::from_str(key))
			.expect("read fetch counter")
			.as_f64()
			.expect("fetch counter is numeric") as u32
	}

	fn set_counter(&self, key: &str, value: u32) {
		Reflect::set(
			js_sys::global().as_ref(),
			&JsValue::from_str(key),
			&JsValue::from_f64(value as f64),
		)
		.expect("write fetch counter");
	}

	fn create_requests(&self) -> u32 {
		self.counter("__reinhardtCreateClusterRequests")
	}

	fn create_bodies(&self) -> Vec<String> {
		Reflect::get(
			js_sys::global().as_ref(),
			&JsValue::from_str("__reinhardtCreateClusterBodies"),
		)
		.expect("read create request bodies")
		.dyn_into::<js_sys::Array>()
		.expect("create request bodies array")
		.iter()
		.map(|value| value.as_string().expect("create request body is text"))
		.collect()
	}

	fn update_requests(&self) -> u32 {
		self.counter("__reinhardtUpdateClusterRequests")
	}

	fn delete_requests(&self) -> u32 {
		self.counter("__reinhardtDeleteClusterRequests")
	}

	fn set_create_requests(&self, value: u32) {
		self.set_counter("__reinhardtCreateClusterRequests", value);
	}

	fn open_delete_gate(&self) {
		let global = js_sys::global();
		Reflect::set(
			global.as_ref(),
			&JsValue::from_str("__reinhardtDeleteClusterGateOpen"),
			&JsValue::TRUE,
		)
		.expect("open delete gate");
		let resolvers = Reflect::get(
			global.as_ref(),
			&JsValue::from_str("__reinhardtDeleteClusterResolvers"),
		)
		.expect("read delete resolvers")
		.dyn_into::<js_sys::Array>()
		.expect("delete resolvers array");
		let response_factory =
			Function::new_no_args("return new Response('null', { status: 200 });");
		for resolver in resolvers.iter() {
			resolver
				.dyn_into::<Function>()
				.expect("resolver function")
				.call1(
					&JsValue::NULL,
					&response_factory
						.call0(&JsValue::NULL)
						.expect("build delete success response"),
				)
				.expect("resolve delete response");
		}
		resolvers.set_length(0);
	}
}

impl Drop for FetchGuard {
	fn drop(&mut self) {
		self.release_page_requests();
		let global = js_sys::global();
		let _ = Reflect::set(
			self.window.as_ref(),
			&JsValue::from_str("fetch"),
			&self.previous_fetch,
		);
		for key in [
			"__reinhardtPageClusterEndpoint",
			"__reinhardtPageClusterResolvers",
			"__reinhardtCreateClusterEndpoint",
			"__reinhardtUpdateClusterEndpoint",
			"__reinhardtDeleteClusterEndpoint",
			"__reinhardtCreateClusterRequests",
			"__reinhardtCreateClusterBodies",
			"__reinhardtUpdateClusterRequests",
			"__reinhardtDeleteClusterRequests",
			"__reinhardtDeleteClusterGateOpen",
			"__reinhardtDeleteClusterResolvers",
		] {
			let _ = Reflect::delete_property(global.as_ref(), &JsValue::from_str(key));
		}
	}
}

fn query_input(root: &web_sys::Element, id: &str) -> web_sys::HtmlInputElement {
	root.query_selector(&format!("#{id}"))
		.expect("query input")
		.expect("input exists")
		.dyn_into::<web_sys::HtmlInputElement>()
		.expect("input element")
}

async fn wait_until(
	fetch: &FetchGuard,
	label: &str,
	mut ready: impl FnMut() -> bool,
	phase: impl Fn() -> String,
) {
	for _ in 0..100 {
		if ready() {
			return;
		}
		TimeoutFuture::new(10).await;
	}
	panic!(
		"{label} timed out after 1000ms; phase={}; create_requests={}; update_requests={}; delete_requests={}",
		phase(),
		fetch.create_requests(),
		fetch.update_requests(),
		fetch.delete_requests(),
	);
}

async fn settle_browser() {
	reinhardt_pages::reactive::with_runtime(|runtime| runtime.flush_updates());
	defer_yield().await;
	TimeoutFuture::new(0).await;
	TimeoutFuture::new(0).await;
}

#[wasm_bindgen_test]
#[serial(server_mutation_globals)]
fn hydration_preserves_the_existing_ready_marker_node() {
	let _ssr_state = SsrStateElement::install();
	let root = BodyRoot::new("server-mutation-hydration");
	let scope = ReactiveScope::new();

	scope.enter(|| {
		let id_counter = reinhardt_pages::reactive::hooks::id::id_counter_snapshot();
		let ssr_html = ClusterMutationComponent.render().render_to_string();
		root.element.set_inner_html(&ssr_html);
		reinhardt_pages::reactive::hooks::id::restore_id_counter(id_counter);
		let marker = root
			.element
			.query_selector("#cluster-mutations-ready")
			.expect("query ready marker")
			.expect("SSR marker exists");
		let hydration_root = root
			.element
			.first_element_child()
			.expect("SSR component root exists");

		hydrate(&ClusterMutationComponent, &Element::new(hydration_root))
			.expect("hydrate cluster mutation component");

		assert!(marker.is_connected());
		assert!(
			root.element
				.query_selector("#cluster-mutations-ready")
				.expect("query hydrated ready marker")
				.is_some()
		);
	});

	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn model_form_mutation_surfaces_field_errors_before_public_callbacks_and_then_resets() {
	let root = BodyRoot::new("server-mutation-create");
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let seen_error = Rc::new(RefCell::new(None::<String>));
	let (form, runtime, mutation) = scope.enter(|| {
		let form = cluster_create_form!();
		let runtime = use_form(&form).build();
		form.clone()
			.into_page()
			.mount(&Element::new(root.element.clone()))
			.expect("mount create form");
		let field = form.name_field();
		let runtime_for_callback = runtime.clone();
		let seen_error_for_callback = Rc::clone(&seen_error);
		let mutation = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.on_error(move |_| {
				*seen_error_for_callback.borrow_mut() = runtime_for_callback
					.get_field_state(field)
					.error
					.as_ref()
					.map(|error| error.message().to_owned());
			})
			.build();
		(form, runtime, mutation)
	});

	runtime.set_value(form.name_field(), "existing".to_owned());
	settle_browser().await;
	assert_eq!(
		query_input(&root.element, "cluster-create-form-name").value(),
		"existing"
	);

	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"create validation error",
		|| {
			runtime
				.get_field_state(form.name_field())
				.error
				.as_ref()
				.map(|error| error.message())
				== Some("Name is already used")
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(seen_error.borrow().as_deref(), Some("Name is already used"));

	runtime.set_value(form.name_field(), "fresh".to_owned());
	settle_browser().await;
	assert_eq!(
		query_input(&root.element, "cluster-create-form-name").value(),
		"fresh"
	);

	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"create success result",
		|| {
			mutation
				.result()
				.as_ref()
				.map(|response| response.token.as_str())
				== Some("one-time-token")
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	wait_until(
		&fetch,
		"create form reset",
		|| {
			query_input(&root.element, "cluster-create-form-name")
				.value()
				.is_empty()
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(fetch.create_requests(), 2);
	assert!(!runtime.get_field_state(form.name_field()).is_dirty);

	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn model_form_mutation_prepares_payload_from_the_attached_runtime() {
	let fetch = FetchGuard::install();
	fetch.set_create_requests(1);
	let scope = ReactiveScope::new();
	let (runtime, mutation) = scope.enter(|| {
		let make_form = || cluster_create_form!();
		let receiver_form = make_form();
		let runtime_form = make_form();
		let runtime = use_form(&runtime_form).build();
		receiver_form
			.set_value(
				"name",
				serde_json::Value::String("receiver-value".to_owned()),
			)
			.expect("set receiver form value");
		runtime.set_value(runtime_form.name_field(), "runtime-value".to_owned());
		let mutation = receiver_form.server_mutation(&runtime).build();
		(runtime, mutation)
	});

	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"runtime-owned model form payload",
		|| mutation.result().is_some(),
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	let bodies = fetch.create_bodies();
	assert_eq!(bodies.len(), 1);
	let body: serde_json::Value =
		serde_json::from_str(&bodies[0]).expect("create request body is valid JSON");
	assert_eq!(
		body,
		serde_json::json!({"payload": {"name": "runtime-value"}})
	);
	assert!(runtime.form_state().is_submit_successful.get());
	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn client_form_mutation_separates_exact_and_family_invalidations() {
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let client = QueryClient::new(QueryDefaults::default());
	let detail_requests = Rc::new(Cell::new(0));
	let list_requests = Rc::new(Cell::new(0));
	let seen_error = Rc::new(RefCell::new(None::<String>));
	let (_detail, _list, form, runtime, mutation) = scope.enter(|| {
		let detail = client.observe_for_test(
			CLUSTER_DETAIL_QUERY.query("cluster-1".to_owned(), {
				let detail_requests = Rc::clone(&detail_requests);
				move || {
					let call = detail_requests.get() + 1;
					detail_requests.set(call);
					async move { Ok::<String, String>(format!("detail-{call}")) }
				}
			}),
			QueryOptions::new(),
		);
		let list = client.observe_for_test(
			CLUSTER_LIST_QUERY.query((), {
				let list_requests = Rc::clone(&list_requests);
				move || {
					let call = list_requests.get() + 1;
					list_requests.set(call);
					async move { Ok::<Vec<String>, String>(vec![format!("list-{call}")]) }
				}
			}),
			QueryOptions::new(),
		);
		let form = UpdateClusterRequestClientForm::new().with_defaults(default_update_request());
		let runtime = use_form(&form).build();
		let field = form.name_field();
		let runtime_for_callback = runtime.clone();
		let seen_error_for_callback = Rc::clone(&seen_error);
		let mutation = form
			.server_mutation(&runtime)
			.invalidate(
				client.clone(),
				CLUSTER_DETAIL_QUERY.key("cluster-1".to_owned()),
			)
			.invalidate_family(client.clone(), CLUSTER_LIST_QUERY)
			.on_error(move |_| {
				*seen_error_for_callback.borrow_mut() = runtime_for_callback
					.get_field_state(field)
					.error
					.as_ref()
					.map(|error| error.message().to_owned());
			})
			.build();
		(detail, list, form, runtime, mutation)
	});

	wait_until(
		&fetch,
		"initial query fetches",
		|| detail_requests.get() == 1 && list_requests.get() == 1,
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	runtime.set_value(form.name_field(), "existing".to_owned());
	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"update validation error",
		|| {
			runtime
				.get_field_state(form.name_field())
				.error
				.as_ref()
				.map(|error| error.message())
				== Some("Name is already used")
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(seen_error.borrow().as_deref(), Some("Name is already used"));
	assert_eq!(detail_requests.get(), 1);
	assert_eq!(list_requests.get(), 1);

	runtime.set_value(form.name_field(), "updated cluster".to_owned());
	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"update success result",
		|| {
			mutation
				.result()
				.as_ref()
				.map(|response| (response.cluster_id.as_str(), response.name.as_str()))
				== Some(("cluster-1", "updated cluster"))
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	wait_until(
		&fetch,
		"query invalidations",
		|| detail_requests.get() == 2 && list_requests.get() == 2,
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(fetch.update_requests(), 2);

	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn delete_mutation_suppresses_duplicate_dispatches_while_pending() {
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let success_calls = Rc::new(Cell::new(0));
	let mutation = scope.enter(|| {
		use_server_mutation(delete_cluster::mutation())
			.on_success({
				let success_calls = Rc::clone(&success_calls);
				move |_| success_calls.set(success_calls.get() + 1)
			})
			.build()
	});

	assert_eq!(
		mutation.dispatch("cluster-1".to_owned()),
		MutationDispatchOutcome::Dispatched
	);
	assert_eq!(
		mutation.dispatch("cluster-1".to_owned()),
		MutationDispatchOutcome::AlreadyPending
	);
	wait_until(
		&fetch,
		"delete request start",
		|| fetch.delete_requests() == 1,
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	fetch.open_delete_gate();
	wait_until(
		&fetch,
		"delete success",
		|| success_calls.get() == 1 && matches!(mutation.result(), Some(())),
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(fetch.delete_requests(), 1);

	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn plain_dispatch_does_not_subscribe_the_calling_effect() {
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let effect_runs = Rc::new(Cell::new(0));
	let (cluster_id, mutation, _effect) = scope.enter(|| {
		let cluster_id = Signal::new("cluster-1".to_owned());
		let cluster_id_for_action = cluster_id;
		let request = delete_cluster::mutation();
		let mutation = use_server_mutation(move |_: ()| {
			let cluster_id = cluster_id_for_action.get();
			request(cluster_id)
		})
		.build();
		let mutation_for_effect = mutation;
		let effect_runs_for_effect = Rc::clone(&effect_runs);
		let effect = Effect::new(move || {
			effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
			assert_eq!(
				mutation_for_effect.dispatch(()),
				MutationDispatchOutcome::Dispatched
			);
		});
		(cluster_id, mutation, effect)
	});

	wait_until(
		&fetch,
		"effect-driven delete request",
		|| fetch.delete_requests() == 1,
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	fetch.open_delete_gate();
	wait_until(
		&fetch,
		"effect-driven delete success",
		|| mutation.result().is_some(),
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	cluster_id.set("cluster-2".to_owned());
	reinhardt_pages::reactive::with_runtime(|runtime| runtime.flush_updates());
	settle_browser().await;

	assert_eq!(effect_runs.get(), 1);
	assert_eq!(fetch.delete_requests(), 1);
	scope.dispose();
}

#[rstest]
#[wasm_bindgen_test]
#[serial(server_mutation_globals)]
fn stale_mutation_dispatch_is_ignored() {
	let scope = ReactiveScope::new();
	let mutation = scope.enter(|| use_server_mutation(delete_cluster::mutation()).build());

	scope.dispose();

	assert_eq!(
		mutation.dispatch("cluster-1".to_owned()),
		MutationDispatchOutcome::AlreadyPending
	);
}

#[rstest]
#[wasm_bindgen_test]
#[serial(server_mutation_globals)]
fn stale_generated_mutation_preserves_an_existing_form_submission() {
	let form_scope = ReactiveScope::new();
	let (form, runtime) = form_scope.enter(|| {
		let form = UpdateClusterRequestClientForm::new().with_defaults(default_update_request());
		let runtime = use_form(&form).build();
		(form, runtime)
	});
	let mutation_scope = ReactiveScope::new();
	let mutation = mutation_scope.enter(|| form.server_mutation(&runtime).build());
	mutation_scope.dispose();
	runtime.form_state().is_submitting.set(true);

	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::AlreadyPending);
	assert!(runtime.form_state().is_submitting.get());
	form_scope.dispose();
}

#[rstest]
#[wasm_bindgen_test]
#[serial(server_mutation_globals)]
fn generated_mutation_pending_falls_back_after_form_scope_disposal() {
	let form_scope = ReactiveScope::new();
	let (form, runtime) = form_scope.enter(|| {
		let form = UpdateClusterRequestClientForm::new().with_defaults(default_update_request());
		let runtime = use_form(&form).build();
		(form, runtime)
	});
	let mutation_scope = ReactiveScope::new();
	let mutation = mutation_scope.enter(|| form.server_mutation(&runtime).build());
	form_scope.dispose();

	assert!(!mutation.is_pending());
	mutation_scope.dispose();
}

#[rstest]
#[serial(server_mutation_globals)]
#[test_attr(wasm_bindgen_test)]
async fn generated_dispatch_does_not_subscribe_the_calling_effect() {
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let effect_runs = Rc::new(Cell::new(0));
	let (form, runtime, mutation, _effect) = scope.enter(|| {
		let form = UpdateClusterRequestClientForm::new().with_defaults(default_update_request());
		let runtime = use_form(&form).build();
		runtime.set_value(form.name_field(), "existing".to_owned());
		let mutation = form.server_mutation(&runtime).build();
		let mutation_for_effect = mutation.clone();
		let effect_runs_for_effect = Rc::clone(&effect_runs);
		let effect = Effect::new(move || {
			let run = effect_runs_for_effect.get();
			effect_runs_for_effect.set(run + 1);
			if run == 0 {
				assert_eq!(
					mutation_for_effect.dispatch(),
					MutationDispatchOutcome::Dispatched
				);
			}
		});
		(form, runtime, mutation, effect)
	});

	wait_until(
		&fetch,
		"effect-driven validation error",
		|| {
			runtime
				.get_field_state(form.name_field())
				.error
				.as_ref()
				.map(|error| error.message())
				== Some("Name is already used")
		},
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	settle_browser().await;

	assert_eq!(effect_runs.get(), 1);
	assert_eq!(fetch.update_requests(), 1);
	scope.dispose();
}

#[rstest]
#[serial(server_mutation_globals)]
#[test_attr(wasm_bindgen_test)]
async fn disposing_mutation_scope_clears_parent_form_pending() {
	let _fetch = FetchGuard::install();
	let parent_scope = ReactiveScope::new();
	let (form, runtime) = parent_scope.enter(|| {
		let form = UpdateClusterRequestClientForm::new().with_defaults(default_update_request());
		let runtime = use_form(&form).build();
		runtime.set_value(form.name_field(), "pending".to_owned());
		(form, runtime)
	});
	let mutation_scope = ReactiveScope::new();
	let mutation = mutation_scope.enter(|| form.server_mutation(&runtime).build());

	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	assert!(runtime.form_state().is_submitting.get());
	mutation_scope.dispose();
	settle_browser().await;

	assert!(!runtime.form_state().is_submitting.get());
	parent_scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn browser_preserves_the_fixed_success_hook_order() {
	let root = BodyRoot::new("server-mutation-order");
	let fetch = FetchGuard::install();
	fetch.set_create_requests(1);
	let scope = ReactiveScope::new();
	let client = QueryClient::new(QueryDefaults::default());
	let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
	let detail_requests = Rc::new(Cell::new(0));
	let list_requests = Rc::new(Cell::new(0));
	let (_detail, _list, form, runtime, mutation) = scope.enter(|| {
		let form = cluster_create_form!();
		let runtime = use_form(&form).build();
		form.clone()
			.into_page()
			.mount(&Element::new(root.element.clone()))
			.expect("mount create form");
		let field = form.name_field();
		let runtime_for_exact = runtime.clone();
		let root_for_exact = root.element.clone();
		let order_for_exact = Rc::clone(&order);
		let detail = client.observe_for_test(
			CLUSTER_DETAIL_QUERY.query("cluster-1".to_owned(), {
				let detail_requests = Rc::clone(&detail_requests);
				move || {
					let call = detail_requests.get() + 1;
					detail_requests.set(call);
					let runtime_for_exact = runtime_for_exact.clone();
					let root_for_exact = root_for_exact.clone();
					let order_for_exact = Rc::clone(&order_for_exact);
					async move {
						if call == 2 {
							assert!(!runtime_for_exact.get_field_state(field).is_dirty);
							assert!(
								query_input(&root_for_exact, "cluster-create-form-name")
									.value()
									.is_empty()
							);
							order_for_exact.borrow_mut().push("reset");
							order_for_exact.borrow_mut().push("exact");
						}
						Ok::<String, String>(format!("detail-{call}"))
					}
				}
			}),
			QueryOptions::new(),
		);
		let list = client.observe_for_test(
			CLUSTER_LIST_QUERY.query((), {
				let list_requests = Rc::clone(&list_requests);
				let order_for_family = Rc::clone(&order);
				move || {
					let call = list_requests.get() + 1;
					list_requests.set(call);
					let order_for_family = Rc::clone(&order_for_family);
					async move {
						if call == 2 {
							order_for_family.borrow_mut().push("family");
						}
						Ok::<Vec<String>, String>(vec![format!("list-{call}")])
					}
				}
			}),
			QueryOptions::new(),
		);
		let order_for_success = Rc::clone(&order);
		let mutation = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.on_success(move |_| {
				order_for_success.borrow_mut().push("user-success");
			})
			.invalidate(
				client.clone(),
				CLUSTER_DETAIL_QUERY.key("cluster-1".to_owned()),
			)
			.invalidate_family(client.clone(), CLUSTER_LIST_QUERY)
			.build();
		(detail, list, form, runtime, mutation)
	});

	wait_until(
		&fetch,
		"initial order query fetches",
		|| detail_requests.get() == 1 && list_requests.get() == 1,
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	runtime.set_value(form.name_field(), "ordered".to_owned());
	settle_browser().await;
	assert_eq!(mutation.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"ordered success hooks",
		|| detail_requests.get() == 2 && list_requests.get() == 2 && order.borrow().len() == 4,
		|| format!("{:?}", mutation.phase()),
	)
	.await;
	assert_eq!(
		order.borrow().as_slice(),
		["user-success", "reset", "exact", "family"]
	);

	scope.dispose();
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn disposing_the_owner_scope_blocks_completion_callbacks_and_side_effects() {
	let fetch = FetchGuard::install();
	let scope = ReactiveScope::new();
	let completion_calls = Rc::new(Cell::new(0));
	let side_effect_calls = Rc::new(Cell::new(0));
	let mutation = scope.enter(|| {
		use_server_mutation(delete_cluster::mutation())
			.on_success({
				let completion_calls = Rc::clone(&completion_calls);
				move |_| completion_calls.set(completion_calls.get() + 1)
			})
			.on_success({
				let side_effect_calls = Rc::clone(&side_effect_calls);
				move |_| side_effect_calls.set(side_effect_calls.get() + 1)
			})
			.build()
	});

	assert_eq!(
		mutation.dispatch("cluster-1".to_owned()),
		MutationDispatchOutcome::Dispatched
	);
	wait_until(
		&fetch,
		"dispose test request start",
		|| fetch.delete_requests() == 1,
		|| format!("{:?}", mutation.phase()),
	)
	.await;

	scope.dispose();
	fetch.open_delete_gate();
	TimeoutFuture::new(20).await;
	TimeoutFuture::new(20).await;
	assert_eq!(completion_calls.get(), 0);
	assert_eq!(side_effect_calls.get(), 0);
}

#[reinhardt_macros::model(
    app_label = "pages",
    table_name = "mutation_page_clusters",
    form(name = ClusterCreateForm, fields(name, api_url)),
    info = false
)]
struct PageCluster {
	#[field(primary_key = true)]
	id: i64,
	org_id: String,
	#[field(max_length = 120)]
	#[form(trim)]
	name: String,
	api_url: String,
	token: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
struct ClusterTokenInfo {
	token: String,
}

#[server_fn(model_form = true)]
async fn create_page_cluster(
	payload: ClusterCreateFormData,
) -> Result<ClusterTokenInfo, ServerFnError> {
	let _ = payload;
	Ok(ClusterTokenInfo {
		token: "native-unreachable".into(),
	})
}

fn edit_input(input: &web_sys::HtmlInputElement, value: &str) {
	input.set_value(value);
	input
		.dispatch_event(&web_sys::Event::new("input").expect("input event"))
		.expect("dispatch input");
}

fn dispatch_submit(form: &web_sys::HtmlFormElement) {
	let event = Function::new_no_args(
		"return new SubmitEvent('submit', {cancelable: true, bubbles: true});",
	)
	.call0(&JsValue::NULL)
	.unwrap()
	.dyn_into::<web_sys::Event>()
	.unwrap();
	assert!(!(form.dispatch_event(&event).expect("dispatch submit")));
	assert!(event.default_prevented());
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn named_mutation_page_preserves_runtime_dom_and_typed_result() {
	let root = BodyRoot::new("named-mutation-page");
	let fetch = FetchGuard::for_page_cluster();
	let scope = ReactiveScope::new();
	let client = QueryClient::new(QueryDefaults::default());
	let runtime_success = Rc::new(Cell::new(0));
	let mutation_success = Rc::new(Cell::new(0));
	let mutation_errors = Rc::new(Cell::new(0));
	let list_requests = Rc::new(Cell::new(0));
	let (_list, form, runtime, action) = scope.enter(|| {
		let list = client.observe_for_test(
			CLUSTER_LIST_QUERY.query((), {
				let calls = list_requests.clone();
				move || {
					calls.set(calls.get() + 1);
					async { Ok::<Vec<String>, String>(Vec::new()) }
				}
			}),
			QueryOptions::new(),
		);
		let form = form! {
			name: CreateClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster,
			overrides: {
				name: {
					label: "Name",
					help_text: "For example: prod-us-east"
				},
				api_url: {
					label: "API URL",
					help_text: "Use the Kubernetes API URL"
				},
			},
		};
		let runtime = use_form(&form)
			.on_submit_success({
				let calls = runtime_success.clone();
				let client = client.clone();
				move |_| {
					calls.set(calls.get() + 1);
					client.invalidate_family(CLUSTER_LIST_QUERY);
				}
			})
			.build();
		let action = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.on_success({
				let calls = mutation_success.clone();
				move |_| calls.set(calls.get() + 1)
			})
			.on_error({
				let calls = mutation_errors.clone();
				move |_| calls.set(calls.get() + 1)
			})
			.build();
		let generated = action.page();
		let result_action = action.clone();
		let dismissal_action = action.clone();
		PageElement::new("section")
			.child(generated)
			.child(Page::reactive(move || {
				result_action
					.result()
					.map(|token: ClusterTokenInfo| {
						PageElement::new("output")
							.attr("id", "created-token")
							.child(token.token)
							.into_page()
					})
					.unwrap_or(Page::Empty)
			}))
			.child(
				PageElement::new("button")
					.attr("type", "button")
					.attr("id", "dismiss-token")
					.child("Dismiss")
					.on(
						reinhardt_pages::event::KnownEvent::Click,
						reinhardt_pages::typed_event_handler::<reinhardt_pages::event::ClickEvent, _>(
							move |_| dismissal_action.reset(),
						),
					),
			)
			.into_page()
			.mount(&Element::new(root.element.clone()))
			.expect("mount page");
		(list, form, runtime, action)
	});
	wait_until(
		&fetch,
		"initial list query",
		|| list_requests.get() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	let form_node = root
		.element
		.query_selector("form")
		.expect("form query")
		.expect("generated form")
		.dyn_into::<web_sys::HtmlFormElement>()
		.expect("form");
	let name = root
		.element
		.query_selector("input[name=name]")
		.expect("name query")
		.expect("name input")
		.dyn_into::<web_sys::HtmlInputElement>()
		.expect("input");
	let api_url = root
		.element
		.query_selector("input[name=api_url]")
		.expect("URL query")
		.expect("URL input")
		.dyn_into::<web_sys::HtmlInputElement>()
		.expect("input");
	let submit = form_node
		.query_selector("button[type=submit]")
		.expect("submit query")
		.expect("submit button")
		.dyn_into::<web_sys::HtmlButtonElement>()
		.expect("button");
	let reset = form_node
		.query_selector("button[type=button]")
		.expect("reset query")
		.expect("reset button")
		.dyn_into::<web_sys::HtmlButtonElement>()
		.expect("button");
	let selected = form_node
		.query_selector_all("input[name]:not([type=hidden])")
		.expect("selected fields");
	let names: Vec<_> = (0..selected.length())
		.map(|index| {
			selected
				.item(index)
				.expect("selected input")
				.dyn_into::<web_sys::HtmlInputElement>()
				.expect("input")
				.name()
		})
		.collect();
	assert_eq!(names, vec!["name", "api_url"]);
	assert_eq!(
		root.element
			.query_selector_all("[name=org_id],[name=token]")
			.expect("server-owned fields")
			.length(),
		0
	);

	// Dispatch a submit event directly to test client validation independently of
	// browser constraint-validation UI.
	dispatch_submit(&form_node);
	settle_browser().await;
	assert_eq!(fetch.create_requests(), 0);
	assert!(!(runtime.form_state().field_errors.get().is_empty()));
	assert_eq!(runtime_success.get(), 0);
	assert_eq!(mutation_success.get(), 0);

	name.focus().expect("focus name");
	edit_input(&name, "  existing  ");
	edit_input(&api_url, "https://kubernetes.example.com:6443");
	assert_eq!(form.value("name"), Some(serde_json::json!("  existing  ")));
	for _ in 0..2 {
		dispatch_submit(&form_node);
	}
	wait_until(
		&fetch,
		"one pending create",
		|| fetch.create_bodies().len() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	settle_browser().await;
	assert_eq!(fetch.create_requests(), 1);
	assert!(action.is_pending());
	assert!(submit.disabled(), "pending button: {}", submit.outer_html());
	assert_eq!(name.value(), "  existing  ");
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&fetch.create_bodies()[0]).unwrap(),
		serde_json::json!({"payload": {
			"name": "existing", "api_url": "https://kubernetes.example.com:6443"
		}})
	);
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"structured failure",
		|| mutation_errors.get() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	settle_browser().await;
	assert_eq!(
		runtime.get_field_state(ClusterCreateFormField::Name).error,
		Some(reinhardt_pages::FieldError::new("Name is already used"))
	);
	assert_eq!(name.get_attribute("aria-invalid").as_deref(), Some("true"));
	let field_message = root
		.element
		.query_selector(&format!("[id='{}-error']", name.id()))
		.expect("field error query")
		.expect("field error");
	assert_eq!(
		field_message.text_content().as_deref(),
		Some("Name is already used")
	);
	let global_message = root
		.element
		.query_selector(".reinhardt-form-global-error")
		.expect("global error query")
		.expect("global error");
	assert_eq!(
		global_message.text_content(),
		runtime.form_state().form_error.get()
	);
	assert_eq!(
		runtime.form_state().form_error.get().as_deref(),
		Some(
			"Validation failed\n_all: Cluster cannot be created\norg_id: Organization is unavailable\nunknown: Unknown field failure"
		)
	);
	assert_eq!(
		global_message
			.query_selector_all("a")
			.expect("global links")
			.length(),
		0
	);
	assert_eq!(runtime_success.get(), 0);
	assert_eq!(mutation_success.get(), 0);
	assert!(!(submit.disabled()));

	edit_input(&name, "fresh");
	dispatch_submit(&form_node);
	wait_until(
		&fetch,
		"second create",
		|| fetch.create_bodies().len() == 2,
		|| format!("{:?}", action.phase()),
	)
	.await;
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"success result",
		|| action.result().is_some(),
		|| format!("{:?}", action.phase()),
	)
	.await;
	wait_until(
		&fetch,
		"invalidated list query",
		|| list_requests.get() == 2,
		|| format!("{:?}", action.phase()),
	)
	.await;
	settle_browser().await;
	assert_eq!(runtime_success.get(), 1);
	assert_eq!(mutation_success.get(), 1);
	assert_eq!(fetch.create_requests(), 2);
	assert_eq!(list_requests.get(), 2);
	assert_eq!(name.value(), "");
	assert_eq!(api_url.value(), "");
	assert!(!(runtime.form_state().is_dirty.get()));
	assert!(!(runtime.form_state().is_touched.get()));
	assert!(runtime.form_state().field_errors.get().is_empty());
	assert_eq!(
		action.result(),
		Some(ClusterTokenInfo {
			token: "one-time-token".into()
		})
	);

	runtime.set_value(ClusterCreateFormField::Name, "another draft".to_owned());
	settle_browser().await;
	assert_eq!(name.value(), "another draft");
	reset.click();
	settle_browser().await;
	assert_eq!(name.value(), "");
	assert_eq!(
		action.result(),
		Some(ClusterTokenInfo {
			token: "one-time-token".into()
		})
	);
	root.element
		.query_selector("#dismiss-token")
		.expect("dismiss query")
		.expect("dismiss button")
		.dyn_into::<web_sys::HtmlButtonElement>()
		.unwrap()
		.click();
	settle_browser().await;
	assert_eq!(action.result(), None);
	assert_eq!(
		root.element
			.query_selector_all("#created-token")
			.unwrap()
			.length(),
		0
	);
	let current_name = root
		.element
		.query_selector("input[name=name]")
		.unwrap()
		.unwrap();
	assert!(name.is_same_node(Some(current_name.as_ref())));
	let current_form = root.element.query_selector("form").unwrap().unwrap();
	assert!(form_node.is_same_node(Some(current_form.as_ref())));
	assert_eq!(
		web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.active_element()
			.map(|element| element.is_same_node(Some(name.as_ref()))),
		Some(true)
	);
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_success_callback_reentry_preserves_next_submission() {
	let root = BodyRoot::new("page-reentry");
	let fetch = FetchGuard::for_page_cluster();
	fetch.set_create_requests(1);
	let scope = ReactiveScope::new();
	let successes = Rc::new(Cell::new(0));
	let next = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
	let (runtime, action) = scope.enter(|| {
		let form = form! {
			name: ReentrantClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster,
		};
		let runtime = use_form(&form).build();
		runtime.set_value(ClusterCreateFormField::Name, "first".to_owned());
		runtime.set_value(
			ClusterCreateFormField::ApiUrl,
			"https://cluster.example".to_owned(),
		);
		let action = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.on_success({
				let calls = successes.clone();
				let next = Rc::downgrade(&next);
				move |_| {
					calls.set(calls.get() + 1);
					if calls.get() == 1 {
						let next = next.upgrade().expect("live reentry callback");
						(next.borrow().as_ref().expect("configured callback"))();
					}
				}
			})
			.build();
		let next_runtime = runtime.clone();
		let next_action = action.clone();
		*next.borrow_mut() = Some(Box::new(move || {
			next_runtime.set_value(ClusterCreateFormField::Name, "second".to_owned());
			assert_eq!(next_action.dispatch(), MutationDispatchOutcome::Dispatched);
		}));
		action
			.page()
			.mount(&Element::new(root.element.clone()))
			.expect("mount page");
		(runtime, action)
	});
	let input = root
		.element
		.query_selector("input[name=name]")
		.unwrap()
		.unwrap()
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();
	assert_eq!(action.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"first request",
		|| fetch.create_bodies().len() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"reentrant request",
		|| fetch.create_bodies().len() == 2,
		|| format!("{:?}", action.phase()),
	)
	.await;
	settle_browser().await;
	assert_eq!(successes.get(), 1);
	assert!(action.is_pending());
	assert_eq!(input.value(), "second");
	assert!(
		runtime
			.get_field_state(ClusterCreateFormField::Name)
			.is_dirty
	);
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&fetch.create_bodies()[1]).unwrap(),
		serde_json::json!({"payload": {
			"name": "second", "api_url": "https://cluster.example"
		}})
	);
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"second success",
		|| successes.get() == 2,
		|| format!("{:?}", action.phase()),
	)
	.await;
	settle_browser().await;
	assert_eq!(fetch.create_bodies().len(), 2);
	assert_eq!(input.value(), "");
	assert_eq!(
		action.result(),
		Some(ClusterTokenInfo {
			token: "one-time-token".into()
		})
	);
	assert!(
		input.is_same_node(Some(
			root.element
				.query_selector("input[name=name]")
				.unwrap()
				.unwrap()
				.as_ref()
		))
	);
}

#[path = "mutation_page_cases.rs"]
mod mutation_page_cases;

#[path = "mutation_page_widgets.rs"]
mod mutation_page_widgets;
