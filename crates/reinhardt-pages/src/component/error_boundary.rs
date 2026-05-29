//! Error boundary component for recoverable rendering and resource failures.
//!
//! [`ErrorBoundary`] renders normal content while tracked resources are healthy,
//! then switches to a fallback view when a tracked source reports an error.

use crate::component::{IntoPage, Page, PageElement};
use crate::reactive::{Resource, ResourceState};

/// Error value rendered by an [`ErrorBoundary`] fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryError {
	message: String,
}

impl BoundaryError {
	/// Creates a new boundary error from a displayable message.
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}

	/// Returns the human-readable error message.
	pub fn message(&self) -> &str {
		&self.message
	}
}

/// Tracks whether a data source currently has a boundary-renderable error.
pub trait ErrorTracker {
	/// Returns the current error, if any.
	fn current_error(&self) -> Option<BoundaryError>;
}

impl<T, E> ErrorTracker for Resource<T, E>
where
	T: Clone + 'static,
	E: Clone + ToString + 'static,
{
	fn current_error(&self) -> Option<BoundaryError> {
		match self.get() {
			ResourceState::Error(err) => Some(BoundaryError::new(err.to_string())),
			ResourceState::Loading | ResourceState::Success(_) => None,
		}
	}
}

type BoxedErrorTracker = Box<dyn ErrorTracker>;

/// Component that isolates tracked errors and renders fallback UI.
pub struct ErrorBoundary {
	fallback_fn: Box<dyn Fn(BoundaryError) -> Page>,
	trackers: Vec<BoxedErrorTracker>,
	content_fn: Box<dyn Fn() -> Page>,
	reset_fn: Box<dyn Fn()>,
}

impl ErrorBoundary {
	/// Creates a boundary with empty content and a paragraph fallback.
	pub fn new() -> Self {
		Self {
			fallback_fn: Box::new(|error| {
				PageElement::new("p")
					.child(error.message().to_string())
					.into_page()
			}),
			trackers: Vec::new(),
			content_fn: Box::new(|| Page::Empty),
			reset_fn: Box::new(|| {}),
		}
	}

	/// Sets the fallback renderer used when a tracked error is present.
	pub fn fallback(mut self, f: impl Fn(BoundaryError) -> Page + 'static) -> Self {
		self.fallback_fn = Box::new(f);
		self
	}

	/// Tracks a [`Resource`] for `ResourceState::Error`.
	pub fn track<T, E>(mut self, resource: Resource<T, E>) -> Self
	where
		T: Clone + 'static,
		E: Clone + ToString + 'static,
	{
		self.trackers.push(Box::new(resource));
		self
	}

	/// Tracks a custom error source.
	pub fn track_custom(mut self, tracker: impl ErrorTracker + 'static) -> Self {
		self.trackers.push(Box::new(tracker));
		self
	}

	/// Sets the content renderer used while no tracked error is present.
	pub fn content(mut self, f: impl Fn() -> Page + 'static) -> Self {
		self.content_fn = Box::new(f);
		self
	}

	/// Sets the callback invoked by [`reset`](Self::reset).
	pub fn on_reset(mut self, f: impl Fn() + 'static) -> Self {
		self.reset_fn = Box::new(f);
		self
	}

	/// Invokes the configured reset callback.
	pub fn reset(&self) {
		(self.reset_fn)();
	}

	/// Returns the first tracked error, if any.
	pub fn current_error(&self) -> Option<BoundaryError> {
		self.trackers
			.iter()
			.find_map(|tracker| tracker.current_error())
	}

	/// Renders either the content or fallback view in a boundary marker.
	pub fn render(&self) -> Page {
		if let Some(error) = self.current_error() {
			PageElement::new("div")
				.attr("data-rh-error-boundary", "error")
				.child((self.fallback_fn)(error))
				.into_page()
		} else {
			PageElement::new("div")
				.attr("data-rh-error-boundary", "ok")
				.child((self.content_fn)())
				.into_page()
		}
	}
}

impl Default for ErrorBoundary {
	fn default() -> Self {
		Self::new()
	}
}

impl IntoPage for ErrorBoundary {
	fn into_page(self) -> Page {
		self.render()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone)]
	struct StaticErrorTracker(Option<BoundaryError>);

	impl ErrorTracker for StaticErrorTracker {
		fn current_error(&self) -> Option<BoundaryError> {
			self.0.clone()
		}
	}

	#[test]
	fn render_uses_content_without_error() {
		let boundary = ErrorBoundary::new()
			.track_custom(StaticErrorTracker(None))
			.content(|| PageElement::new("main").child("ready").into_page());

		assert_eq!(
			boundary.render().render_to_string(),
			r#"<div data-rh-error-boundary="ok"><main>ready</main></div>"#
		);
	}

	#[test]
	fn render_uses_fallback_with_error() {
		let boundary = ErrorBoundary::new()
			.track_custom(StaticErrorTracker(Some(BoundaryError::new("failed"))))
			.fallback(|error| {
				PageElement::new("p")
					.child(error.message().to_string())
					.into_page()
			});

		assert_eq!(
			boundary.render().render_to_string(),
			r#"<div data-rh-error-boundary="error"><p>failed</p></div>"#
		);
	}
}
