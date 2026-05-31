//! Compile-pass: a Signal read inside a `use_effect` closure within a `page!`
//! body whose base identifier IS listed in the deps tuple is accepted
//! (spec §4.5, #4721/#4746). The hook is called via a qualified path so it is
//! exempt from `page!` capture discipline (spec §3.7).

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
			}, (count.clone(), ), );
			"x"
		} }
	});
}
