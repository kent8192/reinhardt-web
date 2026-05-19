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
	// Stub server_fn body. The submit body that would actually `.await`
	// this is wasm-only-gated, so on native we only need the function
	// to exist as a referenceable item.
	pub fn update_profile() {}
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

		// Two-parameter closure: `&Self` and `&T` (the server_fn Ok type).
		// `T` is inferred from the closure body / parameter annotation;
		// the macro never needs to name it. Here we just touch `user_id`
		// to prove the outer-scope capture compiles.
		on_success_ref: |_form, _updated: &i64| {
			let _captured = user_id;
		},

		fields: {
			name: CharField { required },
		},
	};
}
