//! Retained document-head hooks.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use crate::component::Head;
use crate::document_head::{
	DocumentHeadRegistration, current_document_head_manager, report_document_head_error,
};
use crate::reactive::ExplicitDeps;

use super::use_retained_effect;

/// Registers reactive document-head declarations for the mounted scope.
///
/// The registration keeps its original precedence when dependencies change
/// and is removed automatically when the mounted scope is dropped.
pub fn use_head<F>(mut factory: F, deps: ExplicitDeps)
where
	F: FnMut() -> Head + 'static,
{
	let manager = match current_document_head_manager() {
		Ok(manager) => manager,
		Err(error) => {
			report_document_head_error(&error);
			return;
		}
	};
	let registration = Rc::new(RefCell::new(None::<DocumentHeadRegistration>));
	use_retained_effect(
		move || {
			let next = factory();
			let mut registration = registration.borrow_mut();
			let result = match registration.as_ref() {
				Some(token) => token.replace(next),
				None => manager.register_hook(next).map(|token| {
					*registration = Some(token);
				}),
			};
			if let Err(error) = result {
				report_document_head_error(&error);
			}
			None::<fn()>
		},
		deps,
	);
}

/// Registers a reactive document title for the mounted scope.
pub fn use_page_title<F, T>(mut factory: F, deps: ExplicitDeps)
where
	F: FnMut() -> T + 'static,
	T: Into<Cow<'static, str>> + 'static,
{
	use_head(move || Head::new().title(factory()), deps);
}
