//! Issue #4624: the `form!` macro's `on_success:` callback must let the
//! user closure capture locals from the enclosing scope when the closure
//! parameter carries an explicit type annotation.
//!
//! Before the fix, `generate_on_success_callback` spliced the closure
//! literal directly inside the generated `fn submit()` method body, so a
//! closure referencing an outer local like `let target_id = ...;` failed
//! to compile with E0434 ("can't capture dynamic environment in a fn
//! item"). After the fix, when the closure parameter is annotated
//! (`|value: T|`), the literal is expanded at the outer block via
//! `OnSuccessArtifacts::outer_setup`, mirroring the `success_url:` lift
//! from #4623 and the watch-handler lift in `WatchArtifacts` (#4414 fix).
//!
//! Closures without parameter annotations (`|value|`, `|_value|`) keep
//! their historical inline emit in `fn submit()` to preserve backward
//! compatibility for the in-tree callers that do not need outer capture.
//! The companion negative-shape fixture for that branch is the lack of
//! any pre-#4624 trybuild `fail/` fixture — the compile failure used to
//! be the bug itself.

use reinhardt_pages::form;

fn main() {
	// `target_id` lives in this function. Before #4624 was fixed,
	// referencing it from inside an `on_success:` handler failed to
	// compile (E0434) regardless of parameter annotation.
	let target_id: i64 = 42;

	let _form = form! {
		name: OnSuccessCaptureForm,
		server_fn: submit_vote,
		fields: {
			_question_id: IntegerField {
				widget: HiddenInput,
			}
			_choice_id: IntegerField {
				required,
			}
		}
		on_success: |_value: i64| {
			let _captured = target_id;
		},
	};
}

// Mock server function — when called from the form, would normally
// return an `i64`. The closure annotation above must match the function's
// return type so the lift's `__coerce<T, F>` can unify `T`.
fn submit_vote() -> i64 {
	0
}
