//! Issue #4605 / #4612 / #4610: the `form!` macro's `success_url:` attribute
//! must let the user closure capture locals from the enclosing scope.
//!
//! Before the fix, `generate_on_success_callback` spliced the closure
//! literal directly inside the generated `fn submit()` method body, so a
//! closure referencing an outer local like `let qid = ...;` failed to
//! compile with E0434 ("can't capture dynamic environment in a fn item").
//! After the fix, the closure literal is expanded at the outer block via
//! `SuccessUrlArtifacts::outer_setup`, mirroring the watch-handler lift in
//! `WatchArtifacts` (#4414 fix).
//!
//! This fixture validates the fix on the polls.rs end-to-end target shape:
//! a `qid` local that is referenced by the success URL builder.
//!
//! NOTE: depends on the `reinhardt-manouche` parser fix for #4604 / #4611
//! (the `success_url:` arm currently consumes a redundant `Token![:]`).
//! Until that fix lands, this fixture is gated behind a `cfg(any())` so
//! `trybuild` does not attempt to compile it. Remove the `cfg(any())`
//! gate when #4604 / #4611 lands.

// PR1 dependency: this fixture exercises the `success_url: |_form| ...`
// closure shape, which currently fails to parse because the
// `reinhardt-manouche` `success_url:` parser arm consumes a redundant
// `Token![:]` (the outer key-value loop at `parser/form.rs:38` already
// consumed it). The bug is tracked as #4604 / #4611 and is fixed in a
// separate PR. Until that PR lands, the closure-shape branch is gated
// behind `#[cfg(any())]` so trybuild does not attempt to compile it.
//
// To activate this fixture after PR1 merges, replace the `#[cfg(any())]`
// gate with `#[cfg(all())]` (or simply delete the gate and the
// placeholder `main`). The expansion of the lifted `success_url:`
// closure is also indirectly exercised by the `success_url`-touching
// integration test in `tests/use_router_integration.rs` and by the
// `form!` codegen unit tests in `crates/reinhardt-pages/macros/`.

use reinhardt_pages::form;

#[cfg(any())]
fn main() {
	// Enclosing-scope local that the new outer-scope lift must capture.
	let qid: i64 = 42;

	let _form = form! {
		name: SuccessUrlCaptureForm,
		action: "/api/vote",

		state: { loading, error },

		fields: {
			question_id: HiddenField {
				initial: qid.to_string(),
			},
		},

		// The hook that triggered #4605 / #4612. The closure body must be
		// able to reach `qid` — the lift puts the literal at outer scope.
		// After the lift, the closure signature is `|form: &Self|` (the
		// value parameter is no longer threaded through).
		success_url: |_form| format!("/polls/{qid}/results/"),
	};
}

#[cfg(not(any()))]
fn main() {
	// Placeholder so the fixture compiles cleanly before the upstream
	// parser fix in #4604 / #4611 lands.
}
