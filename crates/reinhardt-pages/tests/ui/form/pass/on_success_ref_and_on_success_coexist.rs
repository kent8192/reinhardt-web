//! Issue #4624: `on_success_ref:` and `on_success:` may be supplied
//! together. Documented order: `on_success_ref` (borrows `&value`)
//! runs first, then `on_success` (receives `value` by move).
//!
//! This compile-pass fixture only proves the parser/codegen accept
//! both fields simultaneously; runtime ordering is enforced by the
//! splice order inside the generated `Ok(value) =>` arm.

use reinhardt_pages::form;

mod server_fns {
	// Signature mirrors what `form!` calls at runtime: one positional
	// arg per field + a trailing CSRF arg (auto-injected for POST).
	// Returns a Future-of-Result so the on_success_ref type-safety
	// guard can extract `T = i64`.
	pub async fn update_profile(
		_name: ::std::string::String,
		_csrf: ::std::string::String,
	) -> ::core::result::Result<i64, ::core::convert::Infallible> {
		::core::result::Result::Ok(0)
	}
}

use server_fns::update_profile;

fn main() {
	let user_id: i64 = 7;

	let _form = form! {
		name: DualCallbackForm,
		server_fn: update_profile,

		fields: {
			name: CharField {
				required,
			}
		}

		on_success: |_value| {},
		on_success_ref: |_form, _updated: &i64| {
				let _captured = user_id;
			},

	};
}
