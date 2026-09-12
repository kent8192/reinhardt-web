//! Browser acceptance for named ClientForm views and their existing runtime.
#![cfg(wasm)]

#[path = "../support/client_form_view_browser.rs"]
mod browser;
#[path = "../fixtures/form_scope.rs"]
mod form_scope;
#[path = "../support/client_form_view_shared.rs"]
pub mod support;

use browser::{BrowserRoot, EventListenerGuard, FetchGuard, RouterGuard, flush, wait_for};
use reinhardt_pages::reactive::{Effect, EffectTiming, ReactiveScope};
use reinhardt_pages::{FieldError, FormRuntimeSource, form, use_form};
use serial_test::serial;
use std::{
	cell::{Cell, RefCell},
	rc::Rc,
};
use support::{Choice, LoginRequest, LoginRequestClientForm, ValuesRequestClientForm};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[reinhardt_pages::client_form]
#[derive(Clone, serde::Serialize, reinhardt_core::Validate)]
struct UnvalidatedRequest {
	#[validate(length(min = 1, max = 150))]
	username: String,
}

#[reinhardt_pages::client_form(validate)]
#[derive(Clone, serde::Serialize, reinhardt_core::Validate)]
struct CustomMessageRequest {
	#[validate(length(
		min = 1,
		max = 150,
		message = "Choose a username of 1 to 150 characters"
	))]
	username: String,
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn generated_views_preserve_validation_opt_in_and_custom_messages() {
	form_scope::run(async {
		// Arrange: the same DTO length rule is inactive until the form opts in.
		let submitted = Rc::new(RefCell::new(Vec::new()));
		let definition = UnvalidatedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = reinhardt_pages::use_server_mutation({
			let submitted = submitted.clone();
			move |request: UnvalidatedRequest| {
				submitted.borrow_mut().push(request.username);
				async { Ok::<_, reinhardt_pages::server_fn::ServerFnError>(()) }
			}
		})
		.with_generated_form(&runtime, |runtime| {
			Ok(UnvalidatedRequestClientForm::to_request(runtime))
		})
		.build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: UnvalidatedRequestClientForm,
				mutation: &mutation,
			}
			.into_page(),
		);

		// Act / Assert
		let username = root.input("username", &"😀".repeat(151));
		assert_eq!(username.get_attribute("required"), None);
		root.submit();
		wait_for(|| !mutation.is_pending()).await;
		assert_eq!(*submitted.borrow(), ["😀".repeat(151)]);
		assert_eq!(
			runtime.get_field_state(definition.username_field()).error,
			None
		);
		drop(root);

		// Arrange: opting in retains the existing validator error formatting.
		let definition = CustomMessageRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = reinhardt_pages::use_server_mutation({
			let submitted = submitted.clone();
			move |request: CustomMessageRequest| {
				submitted.borrow_mut().push(request.username);
				async { Ok::<_, reinhardt_pages::server_fn::ServerFnError>(()) }
			}
		})
		.with_generated_form(&runtime, |runtime| {
			Ok(CustomMessageRequestClientForm::to_request(runtime))
		})
		.build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: CustomMessageRequestClientForm,
				mutation: &mutation,
			}
			.into_page(),
		);

		// Act / Assert
		root.input("username", &"😀".repeat(151));
		root.submit();
		assert_eq!(submitted.borrow().len(), 1);
		assert_eq!(
			runtime
				.get_field_state(definition.username_field())
				.error
				.as_ref()
				.map(FieldError::message),
			Some("Custom validation error: Choose a username of 1 to 150 characters")
		);
		assert_eq!(
			root.get::<web_sys::Element>("[aria-live] a")
				.text_content()
				.as_deref(),
			Some("Custom validation error: Choose a username of 1 to 150 characters")
		);
		root.input("username", &"😀".repeat(150));
		root.submit();
		wait_for(|| !mutation.is_pending()).await;
		assert_eq!(*submitted.borrow(), ["😀".repeat(151), "😀".repeat(150)]);
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn configured_success_reset_invalidation_and_navigation_each_run_once() {
	form_scope::run(async {
		use reinhardt_pages::component::Page;
		use reinhardt_pages::reactive::{QueryClient, QueryDefaults, QueryFamily, QueryOptions};
		// Arrange
		let fetch = FetchGuard::install();
		let navigations = Rc::new(Cell::new(0));
		let router = reinhardt_urls::routers::ClientRouter::new()
			.route("home", "/", || Page::text("Home"))
			.route("complete", "/client-form-complete", || {
				Page::text("Complete")
			});
		let _navigation = router.on_navigate({
			let navigations = navigations.clone();
			move |path, _| {
				assert_eq!(path, "/client-form-complete");
				navigations.set(navigations.get() + 1);
			}
		});
		let _router = RouterGuard::install(router);
		let client = QueryClient::new(QueryDefaults::default());
		let query = QueryFamily::<(), String, String>::new("client-form-view.auth");
		let queries = Rc::new(Cell::new(0));
		let _query = client.observe_for_test(
			query.query((), {
				let queries = queries.clone();
				move || {
					queries.set(queries.get() + 1);
					async { Ok(String::from("auth")) }
				}
			}),
			QueryOptions::new(),
		);
		wait_for(|| queries.get() == 1).await;
		let successes = Rc::new(Cell::new(0));
		let definition = LoginRequestClientForm::new();
		let runtime = use_form(&definition)
			.on_submit_success({
				let successes = successes.clone();
				move |_| successes.set(successes.get() + 1)
			})
			.build();
		let mutation = definition
			.server_mutation(&runtime)
			.invalidate(client, query.key(()))
			.redirect("/client-form-complete")
			.reset_form_on_success()
			.build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
			}
			.into_page(),
		);
		root.input("username", "alice");
		root.input("password", "secret");
		// Act
		root.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		fetch.resolve(0, 200, serde_json::json!({"token":"session"}));
		wait_for(|| !mutation.is_pending()).await;
		wait_for(|| queries.get() == 2 && navigations.get() == 1).await;
		// Assert
		assert_eq!(successes.get(), 1);
		assert_eq!(queries.get(), 2);
		assert_eq!(navigations.get(), 1);
		assert_eq!(runtime.get_values().username, "");
		assert_eq!(
			root.get::<web_sys::HtmlInputElement>("input[name=username]")
				.value(),
			""
		);
		assert_eq!(
			reinhardt_pages::app::__current_path_for_test().as_deref(),
			Some("/client-form-complete")
		);
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn pending_submission_retains_nodes_focus_selection_and_callbacks() {
	form_scope::run(async {
		// Arrange
		let fetch = FetchGuard::install();
		let callbacks = Rc::new(RefCell::new(Vec::new()));
		let definition = LoginRequestClientForm::new();
		let runtime = use_form(&definition)
			.on_submit_success({
				let callbacks = callbacks.clone();
				move |_| callbacks.borrow_mut().push("runtime")
			})
			.build();
		let mutation = definition
			.server_mutation(&runtime)
			.on_success({
				let callbacks = callbacks.clone();
				move |_| callbacks.borrow_mut().push("mutation")
			})
			.build();
		let page = form! {
			client_form: LoginRequestClientForm,
			mutation: &mutation,
			id: "login",
			customize: {
				password: { widget: PasswordInput }
			},
			submit: {
				label: "Sign in",
				pending_label: "Signing in..."
			},
		}
		.into_page();
		let root = BrowserRoot::mount(&page);
		let second = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
				id: "second"
			}
			.into_page(),
		);
		let username = root.input("username", "alice");
		root.input("password", "secret");
		username.focus().unwrap();
		username.set_selection_range(1, 3).unwrap();
		let button: web_sys::HtmlButtonElement = root.get("button");
		// Act
		root.submit();
		second.submit();
		root.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		// Assert
		assert_eq!(
			fetch.requests()[0]["body"],
			serde_json::json!({"request": {"username": "alice", "password": "secret"}})
		);
		assert!(mutation.is_pending());
		assert!(button.disabled());
		assert_eq!(button.text_content().as_deref(), Some("Signing in..."));
		assert!(button.is_same_node(Some(&root.get::<web_sys::Node>("button"))));
		assert!(username.is_same_node(Some(&root.get::<web_sys::Node>("input[name=username]"))));
		assert_eq!(username.selection_start().unwrap(), Some(1));
		assert_eq!(username.selection_end().unwrap(), Some(3));
		assert!(
			username.is_same_node(
				web_sys::window()
					.unwrap()
					.document()
					.unwrap()
					.active_element()
					.as_ref()
					.map(|element| element.as_ref())
			)
		);
		fetch.resolve(0, 200, serde_json::json!({"token":"session"}));
		wait_for(|| !mutation.is_pending()).await;
		assert_eq!(*callbacks.borrow(), vec!["runtime", "mutation"]);
		assert!(!button.disabled());
		assert_eq!(button.text_content().as_deref(), Some("Sign in"));
		assert_eq!(fetch.requests().len(), 1);
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn native_controls_submit_one_typed_dto_including_optional_choices() {
	form_scope::run(async {
		let fetch = FetchGuard::install();
		let definition = ValuesRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: ValuesRequestClientForm,
				mutation: &mutation,
				id: "values",
				customize: {
					notes: { widget: Textarea },
					optional_checked: {
						empty_label: "Unset",
						true_label: "Yes",
						false_label: "No"
					}
				},
			}
			.into_page(),
		);
		root.input("text", "hello");
		let notes: web_sys::HtmlTextAreaElement = root.get("textarea");
		notes.set_value("\nnotes");
		notes
			.dispatch_event(&web_sys::Event::new("input").unwrap())
			.unwrap();
		root.input("optional_text", "  trimmed  ");
		root.input("number", "12");
		root.input("optional_number", "1.5");
		assert_eq!(runtime.get_values().optional_number, None);
		assert!(
			definition
				.runtime_custom_widget_error(definition.optional_number_field())
				.is_some()
		);
		root.submit();
		assert_eq!(fetch.requests(), Vec::<serde_json::Value>::new());
		root.input("optional_number", "7");
		let checked: web_sys::HtmlInputElement = root.get("input[name=checked]");
		checked.set_checked(true);
		checked
			.dispatch_event(&web_sys::Event::new("change").unwrap())
			.unwrap();
		for (token, expected) in [
			("choice:0", Some(false)),
			("choice:1", Some(true)),
			("none", None),
		] {
			root.select("optional_checked", token);
			assert_eq!(runtime.get_values().optional_checked, expected);
		}
		root.select("optional_checked", "choice:0");
		root.select("choice", "choice:1");
		root.select("optional_choice", "choice:0");
		assert_eq!(runtime.get_values().optional_choice, Some(Choice::Empty));
		root.select("optional_choice", "none");
		assert_eq!(runtime.get_values().optional_choice, None);
		root.select("optional_choice", "choice:0");
		root.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		assert_eq!(
			fetch.requests()[0]["body"],
			serde_json::json!({"request": {
				"text":"hello","notes":"\nnotes","optional_text":"trimmed","number":12,"optional_number":7,
				"checked":true,"optional_checked":false,"choice":"Active","optional_choice":""
			}})
		);
		fetch.resolve(0, 200, serde_json::json!("saved"));
		wait_for(|| !mutation.is_pending()).await;
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn validation_regions_are_ordered_escaped_and_link_to_retained_controls() {
	form_scope::run(async {
        let fetch = FetchGuard::install();
        let definition = LoginRequestClientForm::new();
        let runtime = use_form(&definition).build();
        let mutation = definition.server_mutation(&runtime).build();
        let root = BrowserRoot::mount(&form! {
	client_form: LoginRequestClientForm,
	mutation: &mutation,
	id: "errors",
	customize: {
		username: {
			help_text: "Visible help",
			aria_describedby: "external errors-help-0"
		}
	},
}.into_page());
        root.submit();
        assert_eq!(fetch.requests(), Vec::<serde_json::Value>::new());
        let username: web_sys::HtmlInputElement = root.get("input[name=username]");
        assert_eq!(username.get_attribute("aria-invalid").as_deref(), Some("true"));
        assert_eq!(username.get_attribute("aria-describedby").as_deref(), Some("errors-help-0 errors-error-0 external"));
        root.input("username", "alice");
        root.input("password", "secret");
        root.submit();
        wait_for(|| fetch.requests().len() == 1).await;
        fetch.resolve(0, 422, serde_json::json!({
            "version":1,"kind":"validation","status":422,"message":"Validation failed",
            "field_errors":[{"field":"password","message":"<password>"},{"field":"username","message":"<username>"},{"field":"unknown","message":"unmatched"}]
        }));
        wait_for(|| !mutation.is_pending()).await;
        let summary: web_sys::Element = root.get("#errors-summary");
        let links = summary.query_selector_all("a").unwrap();
        assert_eq!(links.length(), 2);
        assert_eq!(links.item(0).unwrap().text_content().as_deref(), Some("<username>"));
        assert_eq!(links.item(1).unwrap().text_content().as_deref(), Some("<password>"));
        assert_eq!(summary.query_selector_all("username, password").unwrap().length(), 0);
        assert_eq!(root.0.query_selector_all("[aria-live]").unwrap().length(), 1);
        assert_eq!(runtime.form_state().form_error.get().as_deref(), Some("Validation failed\nunknown: unmatched"));
        root.get::<web_sys::HtmlElement>("#errors-summary a").click();
        assert_eq!(web_sys::window().unwrap().document().unwrap().active_element().unwrap().id(), "errors-field-0");
        runtime.clear_errors();
        flush();
        assert_eq!(summary.text_content().as_deref(), Some(""));
        assert_eq!(username.get_attribute("aria-invalid"), None);
    }).await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn native_reset_batches_bookkeeping_and_respects_cancellation_and_later_writes() {
	form_scope::run(async {
		let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
			username: "initial".into(),
			password: "secret".into(),
		});
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
				id: "reset"
			}
			.into_page(),
		);
		let second = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
				id: "reset-second",
			}
			.into_page(),
		);
		root.input("username", "changed");
		root.input("password", "changed");
		runtime.set_error(definition.username_field(), FieldError::new("Invalid"));
		let bookkeeping = Rc::new(Cell::new(0));
		let state = runtime.form_state();
		let _effect = Effect::new_with_timing(
			{
				let bookkeeping = bookkeeping.clone();
				move || {
					let _ = state.is_touched.get();
					bookkeeping.set(bookkeeping.get() + 1);
				}
			},
			EffectTiming::Layout,
		);
		bookkeeping.set(0);
		let form: web_sys::HtmlFormElement = root.get("form");
		let previous_epoch = definition.runtime_native_reset_epoch();
		assert!(runtime.form_state().is_touched.get());
		form.reset();
		assert_eq!(
			root.get::<web_sys::HtmlInputElement>("input[name=username]")
				.value(),
			"initial"
		);
		wait_for(|| definition.runtime_native_reset_epoch() != previous_epoch).await;
		assert!(!runtime.form_state().is_touched.get());
		assert_eq!(runtime.get_values().username, "initial");
		assert_eq!(runtime.get_values().password, "secret");
		assert_eq!(
			runtime.form_state().field_errors.get(),
			std::collections::HashMap::new()
		);
		assert_eq!(bookkeeping.get(), 1);
		assert_eq!(
			second
				.get::<web_sys::HtmlInputElement>("input[name=username]")
				.value(),
			"initial"
		);
		root.input("username", "before-cancel");
		let cancel =
			EventListenerGuard::new(form.as_ref(), "reset", |event| event.prevent_default());
		form.reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		flush();
		assert_eq!(runtime.get_values().username, "before-cancel");
		drop(cancel);
		form.reset();
		runtime.set_value(definition.username_field(), String::from("later"));
		gloo_timers::future::TimeoutFuture::new(0).await;
		flush();
		assert_eq!(runtime.get_values().username, "later");
		root.input("username", "edited");
		form.reset();
		runtime.reset_field(definition.username_field());
		gloo_timers::future::TimeoutFuture::new(0).await;
		flush();
		assert_eq!(runtime.get_values().username, "initial");
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
fn hydration_preserves_pre_hydration_edits_and_password_omission() {
	ReactiveScope::run(|| {
		let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
			username: "initial".into(),
			password: "secret".into(),
		});
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let page = form! {
			client_form: LoginRequestClientForm,
			mutation: &mutation,
			id: "hydrate",
			customize: {
				password: { widget: PasswordInput }
			}
		}
		.into_page();
		let root = BrowserRoot::new();
		let html = page.render_to_string();
		assert_eq!(html.matches("secret").count(), 0);
		root.0.set_inner_html(&html);
		let username: web_sys::HtmlInputElement = root.get("input[name=username]");
		let password: web_sys::HtmlInputElement = root.get("input[name=password]");
		assert_eq!(password.value(), "");
		username.set_value("typed-before-hydration");
		password.set_value("browser-password");
		root.hydrate(&page);
		assert_eq!(runtime.get_values().username, "typed-before-hydration");
		assert_eq!(runtime.get_values().password, "browser-password");
		root.input("username", "after-hydration");
		assert_eq!(runtime.get_values().username, "after-hydration");
		runtime.set_error(
			definition.username_field(),
			FieldError::new("After hydration"),
		);
		flush();
		assert_eq!(
			root.get::<web_sys::Element>("#hydrate-error-0")
				.text_content()
				.as_deref(),
			Some("After hydration")
		);
		assert_eq!(
			root.get::<web_sys::Element>("#hydrate-summary a")
				.text_content()
				.as_deref(),
			Some("After hydration")
		);
		assert!(username.is_same_node(Some(&root.get::<web_sys::Node>("input[name=username]"))));
	});
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn unicode_length_uses_dto_validation_instead_of_html_utf16_limits() {
	form_scope::run(async {
		let fetch = FetchGuard::install();
		let definition = LoginRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
				id: "unicode"
			}
			.into_page(),
		);
		let username = root.input("username", &"😀".repeat(151));
		root.input("password", "secret");
		assert_eq!(username.get_attribute("maxlength"), None);
		root.submit();
		assert_eq!(fetch.requests(), Vec::<serde_json::Value>::new());
		assert_eq!(
			runtime
				.get_field_state(definition.username_field())
				.error
				.as_ref()
				.map(FieldError::message),
			Some("Length too long: 151 (maximum: 150)")
		);
		root.input("username", &"😀".repeat(150));
		root.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		assert_eq!(
			fetch.requests()[0]["body"]["request"]["username"]
				.as_str()
				.unwrap()
				.chars()
				.count(),
			150
		);
		fetch.resolve(0, 200, serde_json::json!({"token":"ok"}));
		wait_for(|| !mutation.is_pending()).await;
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn disposed_views_ignore_pending_responses_and_deferred_resets() {
	let fetch = FetchGuard::install();
	let callbacks = Rc::new(Cell::new(0));
	let scope = ReactiveScope::new();
	let (root, source) = scope.enter(|| {
		let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
			username: "alice".into(),
			password: "secret".into(),
		});
		let runtime = use_form(&definition)
			.on_submit_success({
				let callbacks = callbacks.clone();
				move |_| callbacks.set(callbacks.get() + 1)
			})
			.build();
		let source = runtime.watch_field::<String>(definition.username_field());
		let mutation = definition.server_mutation(&runtime).build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &mutation,
				id: "dispose"
			}
			.into_page(),
		);
		root.submit();
		(root, source)
	});
	wait_for(|| fetch.requests().len() == 1).await;
	root.get::<web_sys::HtmlFormElement>("form").reset();
	drop(scope);
	drop(root);
	fetch.resolve(0, 200, serde_json::json!({"token":"late"}));
	gloo_timers::future::TimeoutFuture::new(0).await;
	gloo_timers::future::TimeoutFuture::new(0).await;
	assert_eq!(callbacks.get(), 0);
	assert!(source.try_get_untracked().is_err());
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn response_clones_match_the_existing_mutation_without_a_view() {
	form_scope::run(async {
		struct Counted(Rc<Cell<usize>>);
		impl Clone for Counted {
			fn clone(&self) -> Self {
				self.0.set(self.0.get() + 1);
				Self(self.0.clone())
			}
		}
		let mut counts = Vec::new();
		for render in [false, true] {
			let clones = Rc::new(Cell::new(0));
			let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
				username: "alice".into(),
				password: "secret".into(),
			});
			let runtime = use_form(&definition).build();
			let mutation = reinhardt_pages::use_server_mutation({
				let clones = clones.clone();
				move |_: LoginRequest| {
					let clones = clones.clone();
					async move { Ok::<_, reinhardt_pages::server_fn::ServerFnError>(Counted(clones)) }
				}
			})
			.with_generated_form(&runtime, |runtime| {
				Ok(LoginRequestClientForm::to_request(runtime))
			})
			.build();
			let root = render.then(|| {
				BrowserRoot::mount(
					&form! {
						client_form: LoginRequestClientForm,
						mutation: &mutation
					}
					.into_page(),
				)
			});
			assert_eq!(clones.get(), 0);
			if let Some(root) = &root {
				root.submit();
			} else {
				mutation.dispatch();
			}
			wait_for(|| !mutation.is_pending()).await;
			counts.push(clones.get());
			drop(root);
		}
		assert_eq!(counts[0], counts[1]);
		assert!(counts[0] > 0, "the baseline includes response storage");
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn textarea_hydration_and_source_preferred_fields_keep_reset_defaults() {
	form_scope::run(async {
		let definition = ValuesRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mut defaults = ValuesRequestClientForm::to_request(&runtime);
		defaults.notes = String::from("\nnotes");
		let definition = ValuesRequestClientForm::new().with_defaults(defaults);
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let page = form! {
			client_form: ValuesRequestClientForm,
			mutation: &mutation,
			id: "textarea",
			customize: {
				notes: { widget: Textarea }
			}
		}
		.into_page();
		let root = BrowserRoot::new();
		root.0.set_inner_html(&page.render_to_string());
		let notes: web_sys::HtmlTextAreaElement = root.get("textarea");
		assert_eq!(notes.value(), "\nnotes");
		notes.set_value("browser-edit");
		runtime.reset_field(definition.notes_field());
		root.hydrate(&page);
		assert_eq!(notes.value(), "\nnotes");
		assert_eq!(runtime.get_values().notes, "\nnotes");
		notes.set_value("after");
		notes
			.dispatch_event(&web_sys::Event::new("input").unwrap())
			.unwrap();
		assert_eq!(runtime.get_values().notes, "after");
		root.get::<web_sys::HtmlFormElement>("form").reset();
		wait_for(|| runtime.get_values().notes == "\nnotes").await;
		assert_eq!(notes.default_value().unwrap(), "\nnotes");
	})
	.await;
}

#[rstest::rstest]
#[test_attr(wasm_bindgen_test)]
#[serial(client_form_view_dom)]
async fn cloud_shaped_registration_keeps_named_payload_and_independent_login_state() {
	form_scope::run(async {
		let fetch = FetchGuard::install();
		let registered = Rc::new(Cell::new(0));
		let registration = support::RegisterRequestClientForm::new();
		let runtime = use_form(&registration)
			.on_submit_success({
				let registered = registered.clone();
				move |_| registered.set(registered.get() + 1)
			})
			.build();
		let mutation = registration.server_mutation(&runtime).build();
		let root = BrowserRoot::mount(
			&form! {
				client_form: support::RegisterRequestClientForm,
				mutation: &mutation,
				customize: {
					username: {
						autocomplete: "username"
					},
					email: {
						widget: EmailInput,
						autocomplete: "email"
					},
					password: {
						widget: PasswordInput,
						autocomplete: "new-password"
					},
				},
				submit: {
					label: "Create account",
					pending_label: "Creating account..."
				},
			}
			.into_page(),
		);
		let login = LoginRequestClientForm::new();
		let login_runtime = use_form(&login).build();
		let login_mutation = login.server_mutation(&login_runtime).build();
		let other = BrowserRoot::mount(
			&form! {
				client_form: LoginRequestClientForm,
				mutation: &login_mutation
			}
			.into_page(),
		);
		assert_ne!(
			root.get::<web_sys::Element>("form").id(),
			other.get::<web_sys::Element>("form").id()
		);
		root.input("username", "ab");
		root.input("email", "invalid");
		root.input("password", "short");
		root.submit();
		assert_eq!(fetch.requests().len(), 0);
		assert_eq!(runtime.form_state().field_errors.get().len(), 3);
		assert_eq!(login_runtime.form_state().field_errors.get().len(), 0);
		root.input("username", "alice");
		root.input("email", "alice@example.com");
		root.input("password", "long-enough-password");
		root.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		assert!(!login_mutation.is_pending());
		assert!(!other.get::<web_sys::HtmlButtonElement>("button").disabled());
		assert_eq!(
			fetch.requests()[0]["body"],
			serde_json::json!({"request": {
				"username":"alice","email":"alice@example.com","password":"long-enough-password",
			}})
		);
		fetch.resolve(0, 200, serde_json::json!({"token":"registered"}));
		wait_for(|| registered.get() == 1).await;
		assert_eq!(registered.get(), 1);
		assert_eq!(fetch.requests().len(), 1);
	})
	.await;
}
