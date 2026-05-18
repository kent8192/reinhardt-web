//! Regression test for issue #4515: nesting two `Page::reactive` closures
//! that share a `!Copy` capture must continue to produce the rustc-builtin
//! E0507 diagnostic *plus* the "consider cloning the value before moving
//! it into the closure" suggestion.
//!
//! Background: `Signal<T>` (and most reactive primitives) are intentionally
//! `!Copy` so that they can be cheaply `Clone`-d but cannot accidentally be
//! consumed. When users nest `watch { }` blocks that share the same Signal
//! via `move` captures, the outer `move` closure cannot transfer the Signal
//! into the inner `move` closure twice and rustc reports
//! `E0507: cannot move out of value, a captured variable in an Fn closure`.
//!
//! Modern rustc already emits a "help: consider cloning the value before
//! moving it into the closure" suggestion for this case. The framework
//! ships additional flatten-vs-clone guidance in
//! `crates/reinhardt-pages/docs/watch_semantics.md` and in the
//! `Page::reactive` rustdoc, but the in-source diagnostic relies on
//! rustc's built-in note. This file uses a `!Copy` stand-in (independent
//! of the full reactive runtime) so the rustc behavior can be pinned by
//! the `*.stderr` fixture. If a future rustc release silences or weakens
//! the clone suggestion, the fixture diff catches it.
//!
//! See also: `#[diagnostic::on_unimplemented]` was prototyped on a
//! `ReactiveRenderFn` marker trait and removed once empirical testing
//! showed it does not fire for borrow-check errors (only for "trait bound
//! not satisfied" failures).

// reinhardt-fmt: ignore-all

use reinhardt_core::types::page::Page;

#[derive(Clone)]
struct NotCopy(std::rc::Rc<i32>);

impl NotCopy {
	fn get(&self) -> i32 {
		*self.0
	}
}

fn main() {
	let outer_signal = NotCopy(std::rc::Rc::new(1));

	// Outer Page::reactive — must be Fn(): so the closure captures
	// `outer_signal` by move but can be re-invoked. The inner Page::reactive
	// then tries to move `outer_signal` again, which fails because
	// NotCopy is `!Copy`.
	let _page = Page::reactive(move || {
		let _inner = Page::reactive(move || {
			if outer_signal.get() > 0 {
				Page::text("positive")
			} else {
				Page::text("non-positive")
			}
		});
		Page::text("outer")
	});
}
