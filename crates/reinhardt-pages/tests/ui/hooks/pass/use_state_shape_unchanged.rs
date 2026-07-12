//! Compile-pass: pins the `use_state` public shape (spec §4.4 /
//! Issue #4195 open Q3).
//!
//! `use_state(T)` returns `(Signal<T>, SetState<T>)` — the React-style
//! `[state, setState]` tuple-destructure shape that pre-dates Manouche
//! DSL v2. Any future refactor that changes the tuple, the return type,
//! or the callable setter ergonomics will surface here as a compile error.

use std::rc::Rc;

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::{SetState, SetStateExt, use_state};

fn accepts_rc_fn(setter: Rc<dyn Fn(i32)>) {
	setter(2);
}

fn main() {
	// Tuple destructure must work with an explicit Signal<T> annotation
	// on the first slot — this also pins that the first element is
	// `Signal<T>` and not (for example) a newtype wrapper.
	let (count, set_count): (Signal<i32>, SetState<i32>) = use_state(0);

	// The setter must be callable as a function with a single `T` arg.
	set_count(1);

	// The setter remains the public `Rc<dyn Fn(T)>` shape, so callers can
	// pass it through APIs that accept the underlying trait object without an
	// adapter.
	accepts_rc_fn(set_count.clone());
	let as_fn: Rc<dyn Fn(i32)> = set_count.clone();
	as_fn(3);

	// The setter must also support previous-value updates without requiring
	// callers to read the Signal manually.
	set_count.update(|current| current + 1);

	// The state slot must support the standard `.get()` reactive read.
	let _ = count.get();
}
