#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::component::{
	BoundaryError, ErrorBoundary, ErrorTracker, IntoPage, PageElement,
};

#[derive(Clone)]
struct StaticErrorTracker {
	error: Option<BoundaryError>,
}

impl ErrorTracker for StaticErrorTracker {
	fn current_error(&self) -> Option<BoundaryError> {
		self.error.clone()
	}
}

#[test]
fn test_error_boundary_renders_content_without_error() {
	let boundary = ErrorBoundary::new()
		.content(|| PageElement::new("main").child("Loaded").into_page())
		.fallback(|error| {
			PageElement::new("p")
				.child(error.message().to_string())
				.into_page()
		});

	assert_eq!(
		boundary.into_page().render_to_string(),
		r#"<div data-rh-error-boundary="ok"><main>Loaded</main></div>"#
	);
}

#[test]
fn test_error_boundary_renders_resource_error() {
	let tracker = StaticErrorTracker {
		error: Some(BoundaryError::new("load failed")),
	};

	let boundary = ErrorBoundary::new()
		.track_custom(tracker)
		.content(|| PageElement::new("main").child("Loaded").into_page())
		.fallback(|error| {
			PageElement::new("p")
				.child(error.message().to_string())
				.into_page()
		});

	assert_eq!(
		boundary.into_page().render_to_string(),
		r#"<div data-rh-error-boundary="error"><p>load failed</p></div>"#
	);
}

#[test]
fn test_error_boundary_reset_callback_runs() {
	let reset_count = Rc::new(RefCell::new(0));
	let reset_count_for_callback = reset_count.clone();

	let boundary = ErrorBoundary::new().on_reset(move || {
		*reset_count_for_callback.borrow_mut() += 1;
	});

	boundary.reset();

	assert_eq!(*reset_count.borrow(), 1);
}
