//! Browser replacement of handwritten auth controls using external named DTOs.

#[path = "form_scope.rs"]
mod form_scope;
#[path = "browser_support.rs"]
// The shared browser fixture includes helpers for additional in-crate acceptance cases.
#[allow(dead_code)]
mod support;

use crate::pages_api::{form, use_form};
use auth_dto::dto::LoginRequestClientForm;
use std::{cell::Cell, rc::Rc};
use support::{BrowserRoot, FetchGuard, wait_for};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn external_auth_views_keep_payloads_callbacks_and_navigation_errors() {
	form_scope::run(async {
		// Arrange
		let fetch = FetchGuard::install();
		let successes = Rc::new(Cell::new(0));
		let redirects = Rc::new(Cell::new(0));
		let definition = LoginRequestClientForm::new();
		let runtime = use_form(&definition).on_submit_success({
			let successes = successes.clone();
			move |_| successes.set(successes.get() + 1)
		}).build();
		let mutation = definition.server_mutation(&runtime)
			.redirect("http://[")
			.on_redirect_error({
				let redirects = redirects.clone();
				move |_| redirects.set(redirects.get() + 1)
			}).build();
		let login = BrowserRoot::mount(&form! {
			client_form: LoginRequestClientForm,
			mutation: &mutation,
			customize: {
				password: { widget: PasswordInput }
			},
		}.into_page());
		let registrations = Rc::new(Cell::new(0));
		let register = BrowserRoot::mount(&crate::pages::register({
			let registrations = registrations.clone();
			move || registrations.set(registrations.get() + 1)
		}));
		// Act / Assert
		login.input("username", "alice");
		login.input("password", "secret");
		login.submit();
		login.submit();
		wait_for(|| fetch.requests().len() == 1).await;
		assert_eq!(fetch.requests()[0]["body"], serde_json::json!({"request":{"username":"alice","password":"secret"}}));
		fetch.resolve(0, 200, serde_json::json!({"token":"session"}));
		wait_for(|| redirects.get() == 1).await;
		assert_eq!(successes.get(), 1);
		assert_eq!(redirects.get(), 1);
		assert!(!mutation.is_pending());
		assert_eq!(registrations.get(), 0);
		register.input("username", "alice");
		register.input("email", "alice@example.com");
		register.input("password", "long-enough-password");
		register.submit();
		wait_for(|| fetch.requests().len() == 2).await;
		assert_eq!(fetch.requests()[1]["body"], serde_json::json!({"request":{"username":"alice","email":"alice@example.com","password":"long-enough-password"}}));
		fetch.resolve(1, 200, serde_json::json!({"token":"registered"}));
		wait_for(|| registrations.get() == 1).await;
		assert_eq!(registrations.get(), 1);
		assert_eq!(successes.get(), 1);
		assert_eq!(fetch.requests().len(), 2);
	}).await;
}
