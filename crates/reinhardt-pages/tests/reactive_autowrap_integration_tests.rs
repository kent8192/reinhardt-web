//! Integration tests for spec §4.1 — unconditional auto-wrap.
//!
//! Every `{expr}` and every `if` / `for` control-flow expression inside a
//! `page!` body is wrapped in `Page::reactive(move || ...)` at codegen
//! time. These tests pin the runtime behaviour: re-rendering the snapshot
//! after mutating a tracked `Signal` must produce a different HTML string.

#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::component::Page;
use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;
use rstest::rstest;
use serial_test::serial;

fn render(view: &Page) -> String {
	// `Page::render_to_string()` walks the tree, invoking each
	// `Page::Reactive` closure to obtain the current snapshot — this is
	// exactly what spec §4.1 asks the wrap to enable.
	view.render_to_string()
}

#[rstest]
#[serial]
fn expression_rerenders_when_signal_changes() {
	// Arrange
	let count = Signal::new(0_i32);
	let view = page!(|count: Signal<i32>| {
		div {
			{ count.get().to_string() }
		}
	})(count.clone());

	let snapshot_a = render(&view);

	// Act
	count.set(42);

	// Assert
	let snapshot_b = render(&view);
	assert_ne!(
		snapshot_a, snapshot_b,
		"auto-wrap should cause re-render when signal changes",
	);
	assert!(
		snapshot_b.contains("42"),
		"snapshot_b should contain the new value, got: {snapshot_b}",
	);
}

#[rstest]
#[serial]
fn if_branch_rerenders_when_condition_signal_changes() {
	// Arrange
	let flag = Signal::new(false);
	let view = page!(|flag: Signal<bool>| {
		div {
			if flag.get() {
				p {
					"ON"
				}
			}
		}
	})(flag.clone());

	let snapshot_off = render(&view);

	// Act
	flag.set(true);

	// Assert
	let snapshot_on = render(&view);
	assert!(
		snapshot_on.contains("ON"),
		"snapshot_on should contain ON, got: {snapshot_on}",
	);
	assert_ne!(snapshot_off, snapshot_on);
}

#[rstest]
#[serial]
fn for_loop_rerenders_when_items_signal_changes() {
	// Arrange
	let items = Signal::new(vec![1_i32, 2, 3]);
	let view = page!(|items: Signal<Vec<i32>>| {
		ul {
			let items_val = items.get(); for x in items_val.iter() { li { {x.to_string()} } }
		}
	})(items.clone());

	let snapshot_a = render(&view);

	// Act
	items.set(vec![10, 20, 30, 40]);

	// Assert
	let snapshot_b = render(&view);
	assert_ne!(snapshot_a, snapshot_b);
	assert!(
		snapshot_b.contains("40"),
		"snapshot_b should contain 40, got: {snapshot_b}",
	);
}
