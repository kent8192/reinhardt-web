//! Generated-page variants of the upload and nullable-default contracts.

use super::*;
use reinhardt_pages::component::{Component, Page};
use reinhardt_pages::hydration::hydrate;

fn input(root: &web_sys::Element, name: &str) -> web_sys::HtmlInputElement {
	root.query_selector(&format!("input[name='{name}']"))
		.unwrap()
		.unwrap()
		.dyn_into()
		.unwrap()
}

async fn settle() {
	reinhardt_pages::reactive::with_runtime(|runtime| runtime.flush_updates());
	defer_yield().await;
	TimeoutFuture::new(0).await;
	TimeoutFuture::new(0).await;
}

#[wasm_bindgen_test(async)]
#[serial(model_form_file_upload_globals)]
async fn page_default_clear_restores_configured_baseline() {
	// Arrange
	let root = BodyRoot::new();
	let fetch = SuccessfulFetchGuard::install();
	let scope = ReactiveScope::new();
	let (form, runtime, action) = scope.enter(|| {
		let form = form! {
			name: ClearDefaultPageForm,
			model_form: NullableDefaultTrueForm,
			server_fn: save_nullable_default_true,
		};
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		action.page().mount(&Element::new(root.0.clone())).unwrap();
		(form, runtime, action)
	});
	let published = input(&root.0, "published");
	let clear = input(&root.0, "__reinhardt_defaulted_published");
	assert!(published.checked());
	assert!(!(clear.checked()));

	// Act
	clear.click();
	settle().await;
	assert!(clear.checked());
	assert_eq!(form.value("published"), Some(serde_json::Value::Null));
	assert_eq!(action.dispatch(), MutationDispatchOutcome::Dispatched);
	for _ in 0..100 {
		if action.result().is_some() {
			break;
		}
		TimeoutFuture::new(10).await;
	}
	assert_eq!(action.result(), Some(()));
	assert_eq!(
		fetch.payload(),
		serde_json::json!({"payload":{"published":null}})
	);
	runtime.set_error(
		NullableDefaultTrueFormField::Published,
		FieldError::new("clear this error"),
	);
	root.0
		.query_selector("button[type=button]")
		.unwrap()
		.unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>()
		.unwrap()
		.click();
	settle().await;

	// Assert
	assert_eq!(form.value("published"), None);
	assert!(!(clear.checked()), "after reset: {}", root.0.inner_html());
	assert!(published.checked());
	assert!(!(runtime.form_state().is_dirty.get()));
	assert!(!(runtime.form_state().is_touched.get()));
	assert!(runtime.form_state().field_errors.get().is_empty());
	assert_eq!(action.result(), Some(()));
	assert!(clear.is_same_node(Some(
		input(&root.0, "__reinhardt_defaulted_published").as_ref()
	)));
}

struct UploadPage(Page);
impl Component for UploadPage {
	fn render(&self) -> Page {
		self.0.clone()
	}
	fn name() -> &'static str {
		"UploadPage"
	}
}

#[wasm_bindgen_test(async)]
#[serial(model_form_file_upload_globals)]
async fn page_file_controls_use_the_existing_single_file_channel() {
	// Arrange
	let root = BodyRoot::new();
	let document_file = browser_file("report.pdf");
	let avatar_file = browser_file("avatar.png");
	let fetch = MultipartFetchGuard::install(&document_file, &avatar_file);
	fetch.set_status(200.0);
	let scope = ReactiveScope::new();
	let (form, runtime, action, page) = scope.enter(|| {
		let form = form! {
			name: UploadMutationPageForm,
			model: Upload,
			policy: UploadPolicy,
			fields: [title, document, avatar],
			server_fn: upload,
		};
		form.set_value("title", serde_json::json!("Report"))
			.unwrap();
		let runtime = use_form(&form).build();
		let action = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.build();
		let page = UploadPage(action.page());
		root.0.set_inner_html(&page.render().render_to_string());
		let ssr_state = root
			.0
			.owner_document()
			.unwrap()
			.create_element("script")
			.unwrap();
		ssr_state.set_id("ssr-state");
		ssr_state.set_text_content(Some("{}"));
		root.0.append_child(&ssr_state).unwrap();
		(form, runtime, action, page)
	});
	let document = input(&root.0, "document");
	let avatar = input(&root.0, "avatar");
	select_file(&document, &document_file);
	select_file(&avatar, &avatar_file);
	let html = page.render().render_to_string();
	assert_eq!(html.matches("report.pdf").count(), 0);
	assert_eq!(html.matches("avatar.png").count(), 0);
	assert_eq!(document.get_attribute("value"), None);
	assert_eq!(avatar.get_attribute("value"), None);

	// Act: adopt existing browser selections during hydration, then submit.
	scope.enter(|| hydrate(&page, &Element::new(query_form(&root.0).into())).unwrap());
	settle().await;
	assert_eq!(file_count(&document), 1);
	assert_eq!(file_count(&avatar), 1);
	assert!(runtime.get_field_state(form.document_field()).is_dirty);
	assert_eq!(action.dispatch(), MutationDispatchOutcome::Dispatched);
	wait_for_requests(&fetch, 1).await;
	for _ in 0..100 {
		if action.result().is_some() {
			break;
		}
		TimeoutFuture::new(10).await;
	}
	settle().await;

	// Assert: the multipart spy checks exact JSON fields and File identity.
	assert_eq!(action.result(), Some(()));
	assert_eq!(fetch.payload_error(), None);
	assert_eq!(file_count(&document), 0);
	assert_eq!(file_count(&avatar), 0);
	assert!(!(runtime.get_field_state(form.document_field()).is_dirty));
	assert!(!(runtime.form_state().is_dirty.get()));
	assert!(!(runtime.form_state().is_touched.get()));
	assert_eq!(input(&root.0, "title").value(), "Report");
	assert!(document.is_same_node(Some(input(&root.0, "document").as_ref())));
	select_file(&document, &document_file);
	query_form(&root.0).reset();
	settle().await;
	assert_eq!(file_count(&document), 0);
	assert!(!(runtime.get_field_state(form.document_field()).is_dirty));
	assert_eq!(action.result(), Some(()));
}
