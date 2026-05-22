//! Cross-hook integration tests for the React-aligned `(closure, deps)`
//! signatures introduced in #4195.
//!
//! These tests exercise the **interaction** between hooks (e.g., a Memo
//! consumed by an Effect) under Option A semantics:
//!
//! - Closure runs with no active reactive Observer.
//! - Subscriptions are derived exclusively from the deps tuple.
//! - Cleanup functions run before re-execution and on dispose.

#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_core::reactive::Signal;
use reinhardt_pages::reactive::hooks::{use_effect, use_memo};
use serial_test::serial;

/// Verifies that a Memo wired into an Effect's deps tuple re-runs the
/// Effect when the Memo's listed deps change.
#[test]
#[serial(hooks_deps_integration)]
fn memo_feeding_effect_propagates_listed_dep_changes() {
	// Arrange
	let count = Signal::new(1_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	// Memo doubles the count; listed deps = (count,).
	let count_for_memo = count.clone();
	let doubled = use_memo(move || count_for_memo.get() * 2, (count.clone(),));

	// Effect re-runs when the memo changes; listed deps = (doubled,).
	let runs_for_effect = runs.clone();
	let doubled_for_effect = doubled.clone();
	let _eff = use_effect(
		move || {
			let _ = doubled_for_effect.get();
			*runs_for_effect.borrow_mut() += 1;
			None::<fn()>
		},
		(doubled.clone(),),
	);

	// Act — change the upstream Signal; both Memo and Effect must
	// re-evaluate.
	let runs_after_mount = *runs.borrow();
	count.set(5);

	// Flush passive effects.
	reinhardt_core::reactive::runtime::with_runtime(|rt| rt.flush_updates());

	// Assert
	assert_eq!(
		doubled.get(),
		10,
		"memo must recompute when its listed dep changes"
	);
	assert!(
		*runs.borrow() > runs_after_mount,
		"effect must re-run when its listed Memo dep changes"
	);
}

/// Verifies that reading an unlisted Signal inside an Effect does NOT
/// subscribe — this is the Option A invariant.
#[test]
#[serial(hooks_deps_integration)]
fn effect_does_not_subscribe_to_unlisted_signal_read() {
	// Arrange
	let listed = Signal::new(0_i32);
	let unlisted = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	let listed_for_effect = listed.clone();
	let unlisted_for_effect = unlisted.clone();
	let runs_for_effect = runs.clone();

	// Listed deps cover only `listed`. The effect ALSO reads `unlisted`
	// inside its body — but Option A says no auto-track, so this read
	// must not create a subscription.
	let _eff = use_effect(
		move || {
			let _ = listed_for_effect.get();
			let _ = unlisted_for_effect.get();
			*runs_for_effect.borrow_mut() += 1;
			None::<fn()>
		},
		(listed.clone(),),
	);

	let runs_after_mount = *runs.borrow();

	// Act — change ONLY the unlisted Signal.
	unlisted.set(99);
	reinhardt_core::reactive::runtime::with_runtime(|rt| rt.flush_updates());

	// Assert — effect must NOT re-run.
	assert_eq!(
		*runs.borrow(),
		runs_after_mount,
		"unlisted Signal read MUST NOT subscribe under Option A"
	);
}

/// Verifies that the cleanup function returned from an effect closure
/// runs before the next re-execution.
#[test]
#[serial(hooks_deps_integration)]
fn effect_cleanup_runs_before_rerun() {
	// Arrange
	let s = Signal::new(0_i32);
	let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
	let log_for_effect = log.clone();
	let s_for_effect = s.clone();

	let _eff = use_effect(
		move || {
			let _ = s_for_effect.get();
			log_for_effect.borrow_mut().push("run");
			let log_inner = log_for_effect.clone();
			Some(move || log_inner.borrow_mut().push("cleanup"))
		},
		(s.clone(),),
	);

	// Act — trigger one re-run.
	s.set(1);
	reinhardt_core::reactive::runtime::with_runtime(|rt| rt.flush_updates());

	// Assert — cleanup runs between the two `run`s.
	let recorded = log.borrow().clone();
	assert_eq!(recorded, vec!["run", "cleanup", "run"]);
}

/// Verifies that an empty deps tuple `()` makes the effect mount-only —
/// no re-run regardless of which signals change in the environment.
#[test]
#[serial(hooks_deps_integration)]
fn effect_with_empty_deps_is_mount_only() {
	// Arrange
	let s = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));
	let runs_for_effect = runs.clone();
	let s_for_effect = s.clone();

	let _eff = use_effect(
		move || {
			// Even though we read s.get(), `()` deps means no subscriptions.
			let _ = s_for_effect.get();
			*runs_for_effect.borrow_mut() += 1;
			None::<fn()>
		},
		(),
	);

	assert_eq!(*runs.borrow(), 1, "effect must run once at mount");

	// Act — change the signal; effect must not re-run.
	s.set(42);
	reinhardt_core::reactive::runtime::with_runtime(|rt| rt.flush_updates());

	// Assert
	assert_eq!(
		*runs.borrow(),
		1,
		"empty deps `()` must make the effect mount-only"
	);
}
