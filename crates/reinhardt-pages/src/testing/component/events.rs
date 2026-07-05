//! Stable element handles and synthetic event dispatch.

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_core::types::page::{DummyEvent, EventType};

use super::error::EventError;
#[cfg(feature = "msw")]
use super::server_fn_mock;
use super::tree::{NodeId, ScreenInner};

/// Stable handle to an element in a rendered native test screen.
#[derive(Clone)]
pub struct ElementHandle {
	pub(crate) inner: Rc<RefCell<ScreenInner>>,
	pub(crate) node_id: NodeId,
}

impl ElementHandle {
	pub(crate) fn new(inner: Rc<RefCell<ScreenInner>>, node_id: NodeId) -> Self {
		Self { inner, node_id }
	}

	/// Dispatches a click event and panics when dispatch fails.
	pub fn click(&self) {
		self.try_click().expect("click dispatch failed");
	}

	/// Dispatches a click event.
	pub fn try_click(&self) -> Result<(), EventError> {
		self.dispatch(EventType::Click)
	}

	/// Updates the element value and dispatches an input event.
	pub fn input(&self, value: impl Into<String>) {
		self.try_input(value).expect("input dispatch failed");
	}

	/// Updates the element value and dispatches an input event.
	pub fn try_input(&self, value: impl Into<String>) -> Result<(), EventError> {
		self.dispatch_value(EventType::Input, value.into())
	}

	/// Updates the element value and dispatches a change event.
	pub fn change(&self, value: impl Into<String>) {
		self.try_change(value).expect("change dispatch failed");
	}

	/// Updates the element value and dispatches a change event.
	pub fn try_change(&self, value: impl Into<String>) -> Result<(), EventError> {
		self.dispatch_value(EventType::Change, value.into())
	}

	/// Returns the element text content.
	pub fn text(&self) -> String {
		self.inner.borrow().dom.text_content(self.node_id)
	}

	/// Returns the element tag name.
	pub fn tag_name(&self) -> String {
		self.inner
			.borrow()
			.dom
			.element(self.node_id)
			.map(|node| node.tag.clone())
			.unwrap_or_default()
	}

	/// Returns the current internal form value for value-bearing elements.
	pub fn value(&self) -> Option<String> {
		self.inner.borrow().dom.value(self.node_id)
	}

	fn dispatch(&self, event_type: EventType) -> Result<(), EventError> {
		let (handler, scheduler) = {
			let borrowed = self.inner.borrow();
			if !borrowed.dom.contains(self.node_id) {
				return Err(EventError::DetachedElement);
			}
			if borrowed.dom.element(self.node_id).is_none() {
				return Err(EventError::UnsupportedElement);
			}
			(
				borrowed.dom.event_handler(self.node_id, event_type),
				Rc::clone(&borrowed.scheduler),
			)
		};
		#[cfg(feature = "msw")]
		let mocks = self.inner.borrow().mocks.clone();

		let handler = handler.ok_or(EventError::MissingHandler)?;
		#[cfg(feature = "msw")]
		{
			let _mock_scope = server_fn_mock::activate(mocks);
			scheduler.with_current(|| handler(DummyEvent));
		}
		#[cfg(not(feature = "msw"))]
		scheduler.with_current(|| handler(DummyEvent));
		Ok(())
	}

	fn dispatch_value(&self, event_type: EventType, value: String) -> Result<(), EventError> {
		let (handler, scheduler) = {
			let mut borrowed = self.inner.borrow_mut();
			if !borrowed.dom.contains(self.node_id) {
				return Err(EventError::DetachedElement);
			}
			if !borrowed.dom.set_value(self.node_id, value) {
				return Err(EventError::UnsupportedElement);
			}
			(
				borrowed.dom.event_handler(self.node_id, event_type),
				Rc::clone(&borrowed.scheduler),
			)
		};
		#[cfg(feature = "msw")]
		let mocks = self.inner.borrow().mocks.clone();

		let handler = handler.ok_or(EventError::MissingHandler)?;
		#[cfg(feature = "msw")]
		{
			let _mock_scope = server_fn_mock::activate(mocks);
			scheduler.with_current(|| handler(DummyEvent));
		}
		#[cfg(not(feature = "msw"))]
		scheduler.with_current(|| handler(DummyEvent));
		Ok(())
	}
}

impl std::fmt::Debug for ElementHandle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ElementHandle")
			.field("node_id", &self.node_id)
			.finish_non_exhaustive()
	}
}
