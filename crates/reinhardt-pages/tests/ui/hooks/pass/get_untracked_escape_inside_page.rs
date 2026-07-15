//! Compile-pass: reading a Signal through the `get_untracked` escape hatch
//! inside a `page!`-embedded hook closure opts out of dependency tracking, so
//! an empty dependency list is accepted (spec §4.5, #4721/#4746). The hook is called
//! via a qualified path so it is exempt from `page!` capture discipline.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks;
use reinhardt_pages::{deps, page};

fn main() {
	let _ = page!(|count: Signal<i32>| {
		div { {
			hooks::use_effect({
				let count = count.clone();
				move || {
					let _ = count.get_untracked();
					None::<fn() >
				}
			}, deps![], );
			"x"
		} }
	});
}
