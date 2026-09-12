//! Presentation ownership and browser reset/hydration regressions.

use super::*;

fn input(root: &web_sys::Element, name: &str) -> web_sys::HtmlInputElement {
	root.query_selector(&format!("input[name='{name}']"))
		.unwrap()
		.unwrap()
		.dyn_into()
		.unwrap()
}

fn form_node(root: &web_sys::Element) -> web_sys::HtmlFormElement {
	root.query_selector("form")
		.unwrap()
		.unwrap()
		.dyn_into()
		.unwrap()
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_uses_attached_runtime_instead_of_builder_receiver() {
	// Arrange
	let root = BodyRoot::new("page-attached-runtime");
	let fetch = FetchGuard::for_page_cluster();
	fetch.set_create_requests(1);
	let scope = ReactiveScope::new();
	let (first, second, action) = scope.enter(|| {
		let make_form = || {
			form! {
				name: AttachedClusterPageForm,
				model_form: ClusterCreateForm,
				server_fn: create_page_cluster
			}
		};
		let first = make_form();
		let second = make_form();
		first.set_value("name", serde_json::json!("first")).unwrap();
		second
			.set_value("name", serde_json::json!("second"))
			.unwrap();
		second
			.set_value("api_url", serde_json::json!("https://second.example"))
			.unwrap();
		let first_runtime = use_form(&first).build();
		let second_runtime = use_form(&second).build();
		let first_page = first.server_mutation(&first_runtime).build().page();
		let action = first.server_mutation(&second_runtime).build();
		PageElement::new("section")
			.child(
				PageElement::new("div")
					.attr("id", "receiver-page")
					.child(first_page),
			)
			.child(
				PageElement::new("div")
					.attr("id", "attached-page")
					.child(action.page()),
			)
			.into_page()
			.mount(&Element::new(root.element.clone()))
			.unwrap();
		(first, second, action)
	});
	let receiver = root
		.element
		.query_selector("#receiver-page")
		.unwrap()
		.unwrap();
	let attached = root
		.element
		.query_selector("#attached-page")
		.unwrap()
		.unwrap();
	let first_input = input(&receiver, "name");
	let second_input = input(&attached, "name");
	assert_eq!(first_input.value(), "first");
	assert_eq!(second_input.value(), "second");
	assert_ne!(first_input.id(), second_input.id());
	assert_ne!(form_node(&receiver).id(), form_node(&attached).id());
	assert_eq!(
		attached
			.query_selector_all(&format!("label[for='{}']", second_input.id()))
			.unwrap()
			.length(),
		1
	);

	// Act
	edit_input(&second_input, "attached edit");
	dispatch_submit(&form_node(&attached));
	wait_until(
		&fetch,
		"attached request",
		|| fetch.create_bodies().len() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;

	// Assert
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&fetch.create_bodies()[0]).unwrap(),
		serde_json::json!({"payload":{"name":"attached edit","api_url":"https://second.example"}})
	);
	assert_eq!(first.value("name"), Some(serde_json::json!("first")));
	assert_eq!(
		second.value("name"),
		Some(serde_json::json!("attached edit"))
	);
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"attached result",
		|| action.result().is_some(),
		|| format!("{:?}", action.phase()),
	)
	.await;
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_native_reset_retains_mutation_result_and_syncs_metadata() {
	// Arrange
	let root = BodyRoot::new("page-native-reset");
	let fetch = FetchGuard::for_page_cluster();
	fetch.set_create_requests(1);
	let cancel_reset = Rc::new(Cell::new(false));
	let scope = ReactiveScope::new();
	let (runtime, action) = scope.enter(|| {
		let form = form! {
			name: NativeResetClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster
		};
		form.set_value("name", serde_json::json!("initial"))
			.unwrap();
		form.set_value("api_url", serde_json::json!("https://initial.example"))
			.unwrap();
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		let Page::Element(page) = action.page() else {
			panic!("generated form element")
		};
		let cancel = cancel_reset.clone();
		page.on(
			reinhardt_pages::event::KnownEvent::Reset,
			reinhardt_pages::typed_event_handler::<reinhardt_pages::event::ResetEvent, _>(
				move |event: reinhardt_pages::event::ResetEvent| {
					if cancel.get() {
						event.prevent_default();
					}
				},
			),
		)
		.into_page()
		.mount(&Element::new(root.element.clone()))
		.unwrap();
		(runtime, action)
	});
	let name = input(&root.element, "name");
	let element = form_node(&root.element);
	edit_input(&name, "submitted");
	dispatch_submit(&element);
	wait_until(
		&fetch,
		"pending reset request",
		|| fetch.create_bodies().len() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	action.reset();
	assert!(action.is_pending());
	element.reset();
	settle_browser().await;
	assert_eq!(name.value(), name.default_value());
	assert!(action.is_pending());
	assert!(!(runtime.form_state().is_touched.get()));
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"reset result",
		|| action.result().is_some(),
		|| format!("{:?}", action.phase()),
	)
	.await;

	// Act: a canceled native reset preserves edits and errors.
	edit_input(&name, "keep edit");
	runtime.set_error(
		ClusterCreateFormField::Name,
		reinhardt_pages::FieldError::new("keep error"),
	);
	cancel_reset.set(true);
	element.reset();
	settle_browser().await;
	assert_eq!(name.value(), "keep edit");
	assert!(runtime.form_state().is_touched.get());
	assert_eq!(
		runtime.get_field_state(ClusterCreateFormField::Name).error,
		Some(reinhardt_pages::FieldError::new("keep error"))
	);
	cancel_reset.set(false);
	element.reset();
	settle_browser().await;

	// Assert
	assert_eq!(name.value(), name.default_value());
	assert!(!(runtime.form_state().is_dirty.get()));
	assert!(!(runtime.form_state().is_touched.get()));
	assert!(runtime.form_state().field_errors.get().is_empty());
	assert_eq!(
		action.result(),
		Some(ClusterTokenInfo {
			token: "one-time-token".into()
		})
	);
	assert_eq!(fetch.create_requests(), 2);
}

struct RetainedMutationPage(Page);
impl Component for RetainedMutationPage {
	fn render(&self) -> Page {
		self.0.clone()
	}
	fn name() -> &'static str {
		"RetainedMutationPage"
	}
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_hydration_keeps_edited_nodes() {
	// Arrange
	let root = BodyRoot::new("page-hydration");
	let fetch = FetchGuard::for_page_cluster();
	let _state = SsrStateElement::install();
	let scope = ReactiveScope::new();
	let (form, action, component) = scope.enter(|| {
		let form = form! {
			name: HydratedClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster
		};
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		let component = RetainedMutationPage(action.page());
		root.element
			.set_inner_html(&component.render().render_to_string());
		(form, action, component)
	});
	let name = input(&root.element, "name");
	name.set_value("typed before hydration");
	name.focus().unwrap();
	let node = form_node(&root.element);

	// Act
	scope.enter(|| hydrate(&component, &Element::new(node.clone().into())).unwrap());
	settle_browser().await;

	// Assert
	assert!(name.is_same_node(Some(input(&root.element, "name").as_ref())));
	assert_eq!(name.value(), "typed before hydration");
	assert_eq!(
		form.value("name"),
		Some(serde_json::json!("typed before hydration"))
	);
	assert!(node.is_same_node(Some(form_node(&root.element).as_ref())));
	assert_eq!(fetch.create_requests(), 0);
	assert!(!(action.is_pending()));
	assert_eq!(action.result(), None);
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_hydration_with_fresh_runtime_reconciles_instance_ids() {
	// Arrange: server and client construct distinct instances of the same form.
	let root = BodyRoot::new("page-fresh-hydration");
	let fetch = FetchGuard::for_page_cluster();
	let _state = SsrStateElement::install();
	let make_component = || {
		let form = form! {
			name: FreshClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster
		};
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		let component = RetainedMutationPage(action.page());
		(form, action, component)
	};
	{
		let server_scope = ReactiveScope::new();
		server_scope.enter(|| {
			let (_, _, component) = make_component();
			root.element
				.set_inner_html(&component.render().render_to_string());
		});
	}
	let node = form_node(&root.element);
	let name = input(&root.element, "name");
	let server_id = name.id();
	name.set_value("edited before client startup");
	name.focus().unwrap();
	let client_scope = ReactiveScope::new();
	let (form, action, component) = client_scope.enter(make_component);

	// Act
	client_scope.enter(|| hydrate(&component, &Element::new(node.clone().into())).unwrap());
	settle_browser().await;

	// Assert: retained nodes receive the client's unique accessible references.
	assert!(name.is_same_node(Some(input(&root.element, "name").as_ref())));
	assert_ne!(name.id(), server_id);
	assert_eq!(
		root.element
			.query_selector_all(&format!("label[for='{}']", name.id()))
			.unwrap()
			.length(),
		1
	);
	for id in name
		.get_attribute("aria-describedby")
		.unwrap()
		.split_whitespace()
	{
		assert_eq!(
			root.element
				.query_selector_all(&format!("[id='{id}']"))
				.unwrap()
				.length(),
			1
		);
	}
	assert_eq!(name.value(), "edited before client startup");
	assert_eq!(
		form.value("name"),
		Some(serde_json::json!("edited before client startup"))
	);
	assert!(
		web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.active_element()
			.unwrap()
			.is_same_node(Some(name.as_ref()))
	);
	assert_eq!(action.result(), None);
	assert_eq!(fetch.create_requests(), 0);
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_unmount_releases_presentation_resources() {
	// Arrange
	let fetch = FetchGuard::for_page_cluster();
	fetch.set_create_requests(1);
	let app_scope = ReactiveScope::new();
	let (runtime, action) = app_scope.enter(|| {
		let form = form! {
			name: DetachedClusterPageForm,
			model_form: ClusterCreateForm,
			server_fn: create_page_cluster
		};
		form.set_value("name", serde_json::json!("application value"))
			.unwrap();
		form.set_value("api_url", serde_json::json!("https://initial.example"))
			.unwrap();
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		(runtime, action)
	});
	let name = {
		let page_scope = app_scope.enter(ReactiveScope::new);
		let root = BodyRoot::new("page-unmount");
		page_scope.enter(|| {
			action
				.page()
				.mount(&Element::new(root.element.clone()))
				.unwrap()
		});
		let name = input(&root.element, "name");
		assert_eq!(name.value(), "application value");
		name
	};

	// Act
	runtime.set_value(ClusterCreateFormField::Name, "retained runtime".to_owned());
	assert_eq!(action.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_until(
		&fetch,
		"retained action request",
		|| fetch.create_bodies().len() == 1,
		|| format!("{:?}", action.phase()),
	)
	.await;
	fetch.release_page_requests();
	wait_until(
		&fetch,
		"retained action result",
		|| action.result().is_some(),
		|| format!("{:?}", action.phase()),
	)
	.await;
	runtime.reset();
	settle_browser().await;

	// Assert
	assert!(!(name.is_connected()));
	assert_eq!(name.value(), "application value");
	edit_input(&name, "detached event");
	assert!(!(runtime.form_state().is_touched.get()));
	assert_eq!(
		action.result(),
		Some(ClusterTokenInfo {
			token: "one-time-token".into()
		})
	);
	assert_eq!(fetch.create_requests(), 2);
}
