//! Issue #4384: `watch:` handlers must behave as closures and capture
//! enclosing-scope locals.
//!
//! Before the fix, the macro emitted an intermediate `fn __call_watch(...)`
//! item and passed the user's `|form| { ... }` closure as an argument to it,
//! which forced Rust to interpret the handler as a fn item and produced
//! `error[E0434]: can't capture dynamic environment in a fn item` whenever
//! the handler referenced a local from the surrounding function.

use reinhardt_pages::form;

fn main() {
	// `outer_local` lives in this function. Before #4384 was fixed, referencing
	// it from inside a `watch:` handler failed to compile (E0434).
	let outer_local: i64 = 42;

	let _form = form! {
		name: CaptureWatchForm,
		action: "/api/capture",

		state: { loading, error },

		watch: {
			// The handler captures `outer_local` from the enclosing scope.
			// The expression result is used inside the closure body so the
			// compiler must treat it as a real closure (not a fn item).
			captured_view: |_form| {
				let _captured = outer_local;
			},
		},

		fields: {
			content: CharField { required },
		},
	};
}
