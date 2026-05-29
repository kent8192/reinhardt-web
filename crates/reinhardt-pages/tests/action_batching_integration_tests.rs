#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::reactive::{Effect, Signal, batch, with_runtime};
use serial_test::serial;

#[test]
#[serial]
fn test_batch_defers_effect_until_outer_batch_completes() {
	let count = Signal::new(0);
	let log = Rc::new(RefCell::new(Vec::new()));
	let log_for_effect = log.clone();
	let count_for_effect = count.clone();

	let _effect = Effect::new(move || {
		log_for_effect.borrow_mut().push(count_for_effect.get());
	});

	batch(|| {
		count.set(1);
		count.set(2);
		assert_eq!(*log.borrow(), vec![0]);
	});

	with_runtime(|rt| rt.flush_updates());
	assert_eq!(*log.borrow(), vec![0, 2]);
}

#[test]
#[serial]
fn test_nested_batch_flushes_after_outer_batch() {
	let count = Signal::new(0);
	let log = Rc::new(RefCell::new(Vec::new()));
	let log_for_effect = log.clone();
	let count_for_effect = count.clone();

	let _effect = Effect::new(move || {
		log_for_effect.borrow_mut().push(count_for_effect.get());
	});

	batch(|| {
		count.set(1);
		batch(|| {
			count.set(2);
		});
		assert_eq!(*log.borrow(), vec![0]);
	});

	with_runtime(|rt| rt.flush_updates());
	assert_eq!(*log.borrow(), vec![0, 2]);
}
