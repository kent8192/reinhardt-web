//! Native event adapters accept the thread-local captures supported by PageEventHandler.

use std::cell::Cell;
use std::rc::Rc;

use reinhardt_pages::event::ClickEvent;
use reinhardt_pages::{page, raw_async_event_handler, typed_async_event_handler};

fn main() {
	let typed_sync_state = Rc::new(Cell::new(0_u32));
	let _typed_sync = page!(|state: Rc<Cell<u32>>| {
		button {
			@click: move |_| state.set(state.get() + 1),
			"Typed sync"
		}
	})(typed_sync_state);

	let typed_async_state = Rc::new(Cell::new(0_u32));
	let _typed_async = typed_async_event_handler::<ClickEvent, _, _>(move |_| {
		let state = Rc::clone(&typed_async_state);
		async move { state.set(state.get() + 1) }
	});

	let raw_sync_state = Rc::new(Cell::new(0_u32));
	let _raw_sync = page!(|state: Rc<Cell<u32>>| {
		div {
			@custom("editor:commit"): move |_| state.set(state.get() + 1),
		}
	})(raw_sync_state);

	let raw_async_state = Rc::new(Cell::new(0_u32));
	let _raw_async = raw_async_event_handler(move |_| {
		let state = Rc::clone(&raw_async_state);
		async move { state.set(state.get() + 1) }
	});
}
