//! Compile-fail: a Signal read inside a `use_effect` closure written directly
//! in a `page!` body must be listed in the deps tuple (spec §4.5, #4721/#4746).
//! Here `count.get()` is read but the deps tuple is empty `()`, so the hook
//! would silently never re-run — promoted to a hard compile error.
//!
//! The hook is called through a qualified path (`hooks::use_effect`) because
//! `page!` capture discipline (spec §3.7) forbids bare value identifiers that
//! are not declared parameters; a multi-segment path is exempt.

use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks;

fn main() {
	let _ = page!(|count: Signal<i32>| {
		div { {
			hooks::use_effect({
				let count = count.clone();
				move || {
					let _ = count.get();
					None::<fn() >
				}
			}, (), );
			"x"
		} }
	});
}
