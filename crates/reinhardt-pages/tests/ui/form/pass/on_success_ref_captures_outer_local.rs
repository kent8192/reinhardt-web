//! Issue #4624: `on_success_ref:` must capture enclosing-scope locals.
//!
//! `on_success:` (the legacy callback) is expanded inline inside the
//! generated `fn submit()` body, so it cannot reference locals like
//! `user_id` from the surrounding function (E0434).
//!
//! `on_success_ref:` is the lifted variant introduced by this issue —
//! its user closure is expanded at the outer construction block (same
//! lexical scope as the `form!` macro invocation), so capturing outer
//! locals compiles cleanly.

use reinhardt_pages::form;

mod server_fns {
	// Stub server_fn body. The signature must match what `form!` would
	// generate at the call site: one positional arg per form field
	// (`name: String` from the `CharField`) plus a trailing CSRF token
	// arg (auto-injected for the default POST method). Returns an
	// `impl Future<Output = Result<T, E>>` so the compile-time
	// type-safety guard inside the `on_success_ref:` lift (#4624) can
	// extract `T = i64` and force-unify it with the user closure's
	// value parameter type. A real `#[server_fn]` expansion has the
	// same shape.
	pub async fn update_profile(
		_name: ::std::string::String,
		_csrf: ::std::string::String,
	) -> ::core::result::Result<i64, ::core::convert::Infallible> {
		::core::result::Result::Ok(0)
	}
}

use server_fns::update_profile;

fn main() {
	// `user_id` lives in this function. Before #4624, referencing it
	// from inside an `on_success:` closure failed with E0434. The new
	// `on_success_ref:` lifts the closure into this same scope, so the
	// capture works.
	let user_id: i64 = 42;

	let _form = form! {
		name: ProfileForm,
		server_fn: update_profile,

		fields: {
			name: CharField {
				required,
			}
		}

		on_success_ref: |_form, _updated: &i64| {
				let _captured = user_id;
			},

	};
}
