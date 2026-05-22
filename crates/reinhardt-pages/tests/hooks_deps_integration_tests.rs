#![cfg(not(target_arch = "wasm32"))]
//! Integration tests for React-parity hook deps re-run gating (spec §4.2).
//!
//! Verifies that the runtime suppresses re-runs of `use_effect`
//! closures when the firing signal is not in the explicit deps tuple,
//! and re-runs them when it is. PR5 / Issue #4195.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_effect;
use reinhardt_pages::reactive::runtime;
use rstest::rstest;
use serial_test::serial;
use std::cell::RefCell;
use std::rc::Rc;

/// Flush any pending passive effects scheduled via the runtime. Mirrors
/// what `wasm_bindgen_futures::spawn_local` would do in a WASM build.
fn flush_runtime() {
	runtime::with_runtime(|rt| rt.flush_updates());
}

#[rstest]
#[serial]
fn use_effect_does_not_rerun_when_unlisted_signal_changes() {
	// Arrange
	let listed = Signal::new(0_i32);
	let unlisted = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	let _e = use_effect(
		{
			let listed = listed.clone();
			let unlisted = unlisted.clone();
			let runs = Rc::clone(&runs);
			move || {
				let _ = listed.get();
				let _ = unlisted.get();
				*runs.borrow_mut() += 1;
			}
		},
		(listed.clone(),), // only `listed` is a dep
	);

	let initial = *runs.borrow();
	assert_eq!(initial, 1, "effect must run once on mount");

	// Act — change the unlisted signal. Spec §4.2: must NOT re-run.
	unlisted.set(42);
	flush_runtime();

	// Assert
	assert_eq!(
		*runs.borrow(),
		initial,
		"effect re-ran when an unlisted signal changed (deps gating broken)"
	);
}

#[rstest]
#[serial]
fn use_effect_reruns_when_listed_signal_changes() {
	// Arrange
	let listed = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	let _e = use_effect(
		{
			let listed = listed.clone();
			let runs = Rc::clone(&runs);
			move || {
				let _ = listed.get();
				*runs.borrow_mut() += 1;
			}
		},
		(listed.clone(),),
	);

	let initial = *runs.borrow();
	assert_eq!(initial, 1);

	// Act
	listed.set(1);
	flush_runtime();

	// Assert
	assert!(
		*runs.borrow() > initial,
		"effect did not re-run when a listed signal changed"
	);
}

#[rstest]
#[serial]
fn use_effect_mount_only_does_not_rerun_on_any_change() {
	// Arrange — explicit `()` deps means mount-only (spec §4.2).
	let signal = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	let _e = use_effect(
		{
			let signal = signal.clone();
			let runs = Rc::clone(&runs);
			move || {
				let _ = signal.get();
				*runs.borrow_mut() += 1;
			}
		},
		(),
	);

	assert_eq!(
		*runs.borrow(),
		1,
		"mount-only effect must run once on mount"
	);

	// Act — change the signal a few times.
	signal.set(1);
	flush_runtime();
	signal.set(2);
	flush_runtime();

	// Assert — still exactly one run.
	assert_eq!(
		*runs.borrow(),
		1,
		"mount-only effect re-ran after a signal change (mount-only gating broken)"
	);
}

#[rstest]
#[serial]
fn use_effect_with_two_deps_reruns_on_either() {
	// Arrange
	let a = Signal::new(0_i32);
	let b = Signal::new(0_i32);
	let runs = Rc::new(RefCell::new(0_i32));

	let _e = use_effect(
		{
			let a = a.clone();
			let b = b.clone();
			let runs = Rc::clone(&runs);
			move || {
				let _ = a.get();
				let _ = b.get();
				*runs.borrow_mut() += 1;
			}
		},
		(a.clone(), b.clone()),
	);

	let after_mount = *runs.borrow();
	assert_eq!(after_mount, 1);

	// Act — fire `a`
	a.set(1);
	flush_runtime();
	let after_a = *runs.borrow();
	assert!(after_a > after_mount, "effect did not react to `a`");

	// Act — fire `b`
	b.set(1);
	flush_runtime();
	let after_b = *runs.borrow();
	assert!(after_b > after_a, "effect did not react to `b`");
}
