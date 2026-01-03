//! Profile components using React-like hooks
//!
//! Provides profile view and edit form components with hooks-styled state management.
//! Validation is handled server-side via server functions with automatic CSRF protection.

use crate::shared::types::{ProfileResponse, UpdateProfileRequest};
use reinhardt_pages::component::View;
use reinhardt_pages::page;
use reinhardt_pages::reactive::hooks::use_state;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
use {
	crate::server_fn::profile::{fetch_profile, update_profile},
	wasm_bindgen_futures::spawn_local,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::server_fn::profile::fetch_profile;

/// Profile view component using hooks
///
/// Displays user profile information with loading and error states.
/// Uses React-like hooks for state management.
pub fn profile_view(user_id: Uuid) -> View {
	// Hook-styled state management
	let (profile, set_profile) = use_state(None::<ProfileResponse>);
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);

	#[cfg(target_arch = "wasm32")]
	{
		// Clone setters for async use
		let set_profile = set_profile.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			set_loading(true);
			set_error(None);

			match fetch_profile(user_id).await {
				Ok(profile_data) => {
					set_profile(Some(profile_data));
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	// Extract signal values for page! macro
	let loading_state = loading.get();
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let profile_data = profile.get();
	let has_profile = profile.get().is_some();

	// Clone user_id for use in page! macro
	let user_id_str = user_id.to_string();

	page!(|loading_state: bool, error_msg: String, has_error: bool, profile_data: Option<ProfileResponse>, has_profile: bool, user_id_str: String| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-8",
					div {
						class: "card",
						div {
							class: "card-body",
							h2 {
								class: "card-title mb-4",
								"Profile"
							}
							if loading_state {
								div {
									class: "text-center my-4",
									div {
										class: "spinner-border text-primary",
										role: "status",
										span {
											class: "visually-hidden",
											"Loading..."
										}
									}
								}
							}
							if has_error {
								div {
									class: "alert alert-danger",
									role: "alert",
									{ error_msg }
								}
							}
							if has_profile {
								div {
									div {
										class: "mb-3",
										strong {
											"Bio:"
										}
										p {
											{ if let Some(ref data) = profile_data { data.bio.clone().unwrap_or_else(| | "No bio provided".to_string()) } else { String::new() } }
										}
									}
									div {
										class: "mb-3",
										strong {
											"Location:"
										}
										p {
											{ if let Some(ref data) = profile_data { data.location.clone().unwrap_or_else(| | "Not specified".to_string()) } else { String::new() } }
										}
									}
									div {
										class: "mb-3",
										strong {
											"Website:"
										}
										p {
											if let Some(ref data) = profile_data {
												if let Some(ref website) = data.website {
													a {
														href: website.clone(),
														target: "_blank",
														rel: "noopener noreferrer",
														{ website.clone() }
													}
												} else {
													span {
														"No website"
													}
												}
											}
										}
									}
									div {
										class: "mt-4",
										a {
											href: format!("/profile/{}/edit", user_id_str),
											class: "btn btn-primary",
											"Edit Profile"
										}
									}
								}
							}
						}
					}
				}
			}
		}
	})(
		loading_state,
		error_msg,
		has_error,
		profile_data,
		has_profile,
		user_id_str,
	)
}

/// Profile edit component using hooks
///
/// Provides form for editing user profile with:
/// - Initial values loaded from server
/// - HTML5 validation for URL fields
/// - Server-side validation via server functions
/// - Automatic CSRF protection via server function headers
pub fn profile_edit(user_id: Uuid) -> View {
	// Hook-styled state for form fields
	let (bio, set_bio) = use_state(String::new());
	let (avatar_url, set_avatar_url) = use_state(String::new());
	let (location, set_location) = use_state(String::new());
	let (website, set_website) = use_state(String::new());
	let (error, set_error) = use_state(None::<String>);
	let (loading, set_loading) = use_state(false);
	let (success, set_success) = use_state(false);

	// Load current profile data
	#[cfg(target_arch = "wasm32")]
	{
		let set_bio = set_bio.clone();
		let set_avatar_url = set_avatar_url.clone();
		let set_location = set_location.clone();
		let set_website = set_website.clone();

		spawn_local(async move {
			// Fetch profile data for initial values
			if let Ok(profile_data) = fetch_profile(user_id).await {
				set_bio(profile_data.bio.unwrap_or_default());
				set_avatar_url(profile_data.avatar_url.unwrap_or_default());
				set_location(profile_data.location.unwrap_or_default());
				set_website(profile_data.website.unwrap_or_default());
			}
		});
	}

	// Extract signal values for page! macro
	let bio_value = bio.get();
	let avatar_url_value = avatar_url.get();
	let location_value = location.get();
	let website_value = website.get();
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let loading_state = loading.get();
	let success_state = success.get();

	// Clone user_id for use in page! macro
	let user_id_str = user_id.to_string();

	page!(|bio_value: String, avatar_url_value: String, location_value: String, website_value: String, error_msg: String, has_error: bool, loading_state: bool, success_state: bool, user_id_str: String| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-8",
					div {
						class: "card",
						div {
							class: "card-body",
							h2 {
								class: "card-title mb-4",
								"Edit Profile"
							}
							if success_state {
								div {
									class: "alert alert-success",
									role: "alert",
									"Profile updated successfully! Redirecting..."
								}
							}
							if has_error {
								div {
									class: "alert alert-danger",
									role: "alert",
									{ error_msg }
								}
							}
							form {
								@submit: { let set_error = set_error.clone(); let set_loading = set_loading.clone(); let set_success = set_success.clone(); let bio = bio.clone(); let avatar_url = avatar_url.clone(); let location = location.clone(); let website = website.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { event.prevent_default(); let set_error = set_error.clone(); let set_loading = set_loading.clone(); let set_success = set_success.clone(); let bio_value = bio.get(); let avatar_url_value = avatar_url.get(); let location_value = location.get(); let website_value = website.get(); spawn_local(async move { set_loading(true); set_error(None); set_success(false); let request = UpdateProfileRequest { bio : if bio_value.is_empty() { None } else { Some(bio_value) }, avatar_url : if avatar_url_value.is_empty() { None } else { Some(avatar_url_value) }, location : if location_value.is_empty() { None } else { Some(location_value) }, website : if website_value.is_empty() { None } else { Some(website_value) }, }; match update_profile(request).await { Ok(_) => { set_success(true); set_loading(false); if let Some(window) = web_sys::window() { let _ = window.location().set_href(&format!("/profile/{}", user_id)); } } Err(e) => { set_error(Some(e.to_string())); set_loading(false); } } }); } } },
								div {
									class: "mb-3",
									label {
										r#for: "bio",
										class: "form-label",
										"Bio"
									}
									textarea {
										class: "form-control",
										id: "bio",
										name: "bio",
										rows: 3,
										placeholder: "Tell us about yourself",
										@input: { let set_bio = set_bio.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { use wasm_bindgen::JsCast; use web_sys::HtmlTextAreaElement; if let Some(target) = event.target() { if let Ok(textarea) = target.dyn_into ::<HtmlTextAreaElement>() { set_bio(textarea.value()); } } } } },
										{ bio_value.clone() }
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "avatar_url",
										class: "form-label",
										"Avatar URL"
									}
									input {
										r#type: "url",
										class: "form-control",
										id: "avatar_url",
										name: "avatar_url",
										placeholder: "https://example.com/avatar.jpg",
										value: avatar_url_value.clone(),
										@input: { let set_avatar_url = set_avatar_url.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { use wasm_bindgen::JsCast; use web_sys::HtmlInputElement; if let Some(target) = event.target() { if let Ok(input) = target.dyn_into ::<HtmlInputElement>() { set_avatar_url(input.value()); } } } } },
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "location",
										class: "form-label",
										"Location"
									}
									input {
										r#type: "text",
										class: "form-control",
										id: "location",
										name: "location",
										placeholder: "New York, NY",
										value: location_value.clone(),
										@input: { let set_location = set_location.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { use wasm_bindgen::JsCast; use web_sys::HtmlInputElement; if let Some(target) = event.target() { if let Ok(input) = target.dyn_into ::<HtmlInputElement>() { set_location(input.value()); } } } } },
									}
								}
								div {
									class: "mb-3",
									label {
										r#for: "website",
										class: "form-label",
										"Website"
									}
									input {
										r#type: "url",
										class: "form-control",
										id: "website",
										name: "website",
										placeholder: "https://example.com",
										value: website_value.clone(),
										@input: { let set_website = set_website.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { use wasm_bindgen::JsCast; use web_sys::HtmlInputElement; if let Some(target) = event.target() { if let Ok(input) = target.dyn_into ::<HtmlInputElement>() { set_website(input.value()); } } } } },
									}
								}
								div {
									class: "d-grid gap-2 d-md-flex justify-content-md-end",
									a {
										href: format!("/profile/{}", user_id_str),
										class: "btn btn-secondary",
										"Cancel"
									}
									button {
										r#type: "submit",
										class: if loading_state { "btn btn-primary disabled" } else { "btn btn-primary" },
										if loading_state {
											"Saving..."
										} else {
											"Save Changes"
										}
									}
								}
							}
						}
					}
				}
			}
		}
	})(
		bio_value,
		avatar_url_value,
		location_value,
		website_value,
		error_msg,
		has_error,
		loading_state,
		success_state,
		user_id_str,
	)
}
