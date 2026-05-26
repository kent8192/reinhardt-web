//! Issue #4420: `form!` macro must let `initial:` expressions and inner
//! `watch:` callbacks (`submit_button:`, `error_display:`,
//! `success_navigation:`) behave as real closures that:
//!
//! 1. Capture locals from the enclosing scope (no `E0434: can't capture
//!    dynamic environment in a fn item`).
//! 2. Type-infer their `|form|` parameter from the surrounding form struct
//!    (no `E0282: type annotations needed`).
//!
//! PR #4416 fixed the outer `watch:` handler for #4414 but left these
//! sibling sites emitting `fn` items / un-annotated closures. This fixture
//! exercises both shapes simultaneously.

use reinhardt_pages::form;

fn main() {
	// Enclosing-scope locals that the macro must let `initial:` and the inner
	// watch callbacks reach.
	let outer_initial: i64 = 7;
	let outer_label = "hello".to_string();

	let _form = form! {
		name: CaptureFormFourTwoZero,
		action: "/api/capture-4420",
		state: {
			loading,
			error
		},
		fields: {
			// `initial: <expr>` referencing an outer local — previously emitted
			// into `fn new()` and produced E0434.
			counter: HiddenField {
				initial: outer_initial.to_string()
			},
			content: CharField {
				required,
				initial: outer_label.clone()
			},
		},
		watch: {
			// Each of these callbacks invokes a method on `form`. Without the
			// type-inference fix they fail with E0282 because the closure body
			// is type-checked before the eventual call site supplies `&Self`.
			submit_button: |form| {
				let _is_loading = form.loading().get();
			},
			error_display: |form| {
				let _err = form.error().get();
			},
			success_navigation: |form| {
				let _is_loading = form.loading().get();
				let _captured = outer_label.clone();
			},
		},
	};
}
