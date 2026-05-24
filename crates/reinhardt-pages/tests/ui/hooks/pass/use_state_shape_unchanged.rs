//! Compile-pass: pins the `use_state` public shape (spec §4.4 /
//! Issue #4195 open Q3).
//!
//! `use_state(T)` returns `(Signal<T>, SetState<T>)` — the React-style
//! `[state, setState]` tuple-destructure shape that pre-dates Manouche
//! DSL v2. Any future refactor that changes the tuple, the return type,
//! or the setter ergonomics will surface here as a compile error.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_state;

fn main() {
	// Tuple destructure must work with an explicit Signal<T> annotation
	// on the first slot — this also pins that the first element is
	// `Signal<T>` and not (for example) a newtype wrapper.
	let (count, set_count): (Signal<i32>, _) = use_state(0);

	// The setter must be callable as a function with a single `T` arg.
	set_count(1);

	// The state slot must support the standard `.get()` reactive read.
	let _ = count.get();
}
