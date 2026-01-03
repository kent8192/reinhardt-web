//! Authentication components using React-like hooks
//!
//! Provides login and registration form components with hooks-styled state management.
//! Validation is handled server-side via server functions with automatic CSRF protection.

use crate::shared::types::RegisterRequest;
use reinhardt_pages::component::View;
use reinhardt_pages::page;
use reinhardt_pages::reactive::hooks::use_state;

#[cfg(target_arch = "wasm32")]
use {
	crate::client::router::with_router,
	crate::client::state::set_current_user,
	crate::server_fn::auth::{login, register},
	wasm_bindgen::JsCast,
	wasm_bindgen_futures::spawn_local,
	web_sys::HtmlInputElement,
};

/// Login form component using hooks
///
/// Provides email/password login with:
/// - HTML5 validation for required fields and email format
/// - Server-side validation via server functions
/// - Automatic CSRF protection via server function headers
pub fn login_form() -> View {
	// Hook-styled state management
	let (error, set_error) = use_state(None::<String>);
	let (loading, set_loading) = use_state(false);
	// For CSR (Client-Side Rendering), we don't need hydration, so start with true
	let (is_hydrated, _set_is_hydrated) = use_state(true);

	// Note: use_effect for hydration monitoring is commented out for CSR-only mode
	// If SSR support is added in the future, uncomment and modify the logic below:
	// #[cfg(target_arch = "wasm32")]
	// {
	//     let set_is_hydrated = set_is_hydrated.clone();
	//     use_effect(move || {
	//         // Only check hydration if SSR is enabled
	//         if is_ssr_mode() {  // This function needs to be implemented
	//             set_is_hydrated(is_hydration_complete());
	//             let set_is_hydrated_inner = set_is_hydrated.clone();
	//             on_hydration_complete(move |complete| {
	//                 set_is_hydrated_inner(complete);
	//             });
	//         }
	//     });
	// }

	// Extract signal values for page! macro
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let loading_state = loading.get();
	let hydrated = is_hydrated.get();

	page!(|error_msg: String, has_error: bool, loading_state: bool, hydrated: bool| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-6",
					div {
						class: "card",
						div {
							class: "card-body",
							h2 {
								class: "card-title text-center mb-4",
								"Login"
							}
							if has_error {
								div {
									class: "alert alert-danger",
									{ error_msg }
								}
							}
							if ! hydrated {
								div {
									class: "text-center mb-3",
									div {
										class: "spinner-border spinner-border-sm text-primary",
										span {
											class: "visually-hidden",
											"Loading..."
										}
									}
									small {
										class: "text-muted ms-2",
										"Initializing form..."
									}
								}
							}
							form {
								@submit: { let set_error = set_error.clone(); let set_loading = set_loading.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { event.prevent_default(); let set_error = set_error.clone(); let set_loading = set_loading.clone(); let form = event.target().and_then(|t| t.dyn_into ::<web_sys::HtmlFormElement>().ok()); if let Some(form) = form { let email = form.elements().named_item("email").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); let password = form.elements().named_item("password").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); spawn_local(async move { set_loading(true); set_error(None); match login(email, password).await { Ok(user_info) => { set_current_user(Some(user_info)); with_router(|router| { let _ = router.push("/timeline"); }); } Err(e) => { set_error(Some(e.to_string())); set_loading(false); } } }); } } } },
								div {
									class: "mb-3",
									label {
										r#for: "email",
										class: "form-label",
										"Email"
									}
									input {
										r#type: "email",
										class: "form-control",
										id: "email",
										name: "email",
										placeholder: "Enter your email",
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "password",
										class: "form-label",
										"Password"
									}
									input {
										r#type: "password",
										class: "form-control",
										id: "password",
										name: "password",
										placeholder: "Enter your password",
									}
								}
								div {
									class: "d-grid",
									button {
										r#type: "submit",
										class: if loading_state || ! hydrated { "btn btn-primary disabled" } else { "btn btn-primary" },
										if ! hydrated {
											"Loading..."
										} else if loading_state {
											"Logging in..."
										} else {
											"Login"
										}
									}
								}
								div {
									class: "text-center mt-3",
									"Don't have an account? "
									a {
										href: "/register",
										"Register here"
									}
								}
							}
						}
					}
				}
			}
		}
	})(error_msg, has_error, loading_state, hydrated)
}

/// Registration form component using hooks
///
/// Provides username/email/password registration with:
/// - HTML5 validation for required fields and email format
/// - Server-side validation including password matching
/// - Automatic CSRF protection via server function headers
pub fn register_form() -> View {
	// Hook-styled state management
	let (error, set_error) = use_state(None::<String>);
	let (loading, set_loading) = use_state(false);
	// For CSR (Client-Side Rendering), we don't need hydration, so start with true
	let (is_hydrated, _set_is_hydrated) = use_state(true);

	// Note: use_effect for hydration monitoring is commented out for CSR-only mode
	// If SSR support is added in the future, uncomment and modify the logic below:
	// #[cfg(target_arch = "wasm32")]
	// {
	//     let set_is_hydrated = set_is_hydrated.clone();
	//     use_effect(move || {
	//         // Only check hydration if SSR is enabled
	//         if is_ssr_mode() {  // This function needs to be implemented
	//             set_is_hydrated(is_hydration_complete());
	//             let set_is_hydrated_inner = set_is_hydrated.clone();
	//             on_hydration_complete(move |complete| {
	//                 set_is_hydrated_inner(complete);
	//             });
	//         }
	//     });
	// }

	// Extract signal values for page! macro
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let loading_state = loading.get();
	let hydrated = is_hydrated.get();

	page!(|error_msg: String, has_error: bool, loading_state: bool, hydrated: bool| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-6",
					div {
						class: "card",
						div {
							class: "card-body",
							h2 {
								class: "card-title text-center mb-4",
								"Register"
							}
							if has_error {
								div {
									class: "alert alert-danger",
									{ error_msg }
								}
							}
							if ! hydrated {
								div {
									class: "text-center mb-3",
									div {
										class: "spinner-border spinner-border-sm text-primary",
										span {
											class: "visually-hidden",
											"Loading..."
										}
									}
									small {
										class: "text-muted ms-2",
										"Initializing form..."
									}
								}
							}
							form {
								@submit: { let set_error = set_error.clone(); let set_loading = set_loading.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { event.prevent_default(); let set_error = set_error.clone(); let set_loading = set_loading.clone(); let form = event.target().and_then(|t| t.dyn_into ::<web_sys::HtmlFormElement>().ok()); if let Some(form) = form { let username = form.elements().named_item("username").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); let email = form.elements().named_item("email").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); let password = form.elements().named_item("password").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); let password_confirmation = form.elements().named_item("password_confirmation").and_then(|e| e.dyn_into ::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default(); if password!= password_confirmation { set_error(Some("Passwords do not match".to_string())); return; } spawn_local(async move { set_loading(true); set_error(None); let request = RegisterRequest { username, email, password, password_confirmation, }; match register(request).await { Ok(()) => { with_router(|router| { let _ = router.push("/login"); }); } Err(e) => { set_error(Some(e.to_string())); set_loading(false); } } }); } } } },
								div {
									class: "mb-3",
									label {
										r#for: "username",
										class: "form-label",
										"Username"
									}
									input {
										r#type: "text",
										class: "form-control",
										id: "username",
										name: "username",
										placeholder: "Choose a username",
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "email",
										class: "form-label",
										"Email"
									}
									input {
										r#type: "email",
										class: "form-control",
										id: "email",
										name: "email",
										placeholder: "Enter your email",
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "password",
										class: "form-label",
										"Password"
									}
									input {
										r#type: "password",
										class: "form-control",
										id: "password",
										name: "password",
										placeholder: "Choose a password",
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "password_confirmation",
										class: "form-label",
										"Confirm Password"
									}
									input {
										r#type: "password",
										class: "form-control",
										id: "password_confirmation",
										name: "password_confirmation",
										placeholder: "Confirm your password",
									}
								}
								div {
									class: "d-grid",
									button {
										r#type: "submit",
										class: if loading_state || ! hydrated { "btn btn-primary disabled" } else { "btn btn-primary" },
										if ! hydrated {
											"Loading..."
										} else if loading_state {
											"Registering..."
										} else {
											"Register"
										}
									}
								}
								div {
									class: "text-center mt-3",
									"Already have an account? "
									a {
										href: "/login",
										"Login here"
									}
								}
							}
						}
					}
				}
			}
		}
	})(error_msg, has_error, loading_state, hydrated)
}
