//! Error types for native component testing.

use std::fmt;

/// Error returned when a DOM query cannot identify one element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
	/// No matching element was found.
	NotFound,
	/// More than one matching element was found.
	MultipleMatches,
}

impl fmt::Display for QueryError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NotFound => write!(f, "no matching element found"),
			Self::MultipleMatches => write!(f, "multiple matching elements found"),
		}
	}
}

impl std::error::Error for QueryError {}

/// Error returned when a synthetic event cannot be dispatched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventError {
	/// The handle no longer points at a node in the screen tree.
	DetachedElement,
	/// The element has no handler for the requested event.
	MissingHandler,
	/// The requested event is not supported for this node.
	UnsupportedElement,
}

impl fmt::Display for EventError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DetachedElement => write!(f, "element is detached from the screen"),
			Self::MissingHandler => write!(f, "element has no handler for the requested event"),
			Self::UnsupportedElement => write!(f, "event is not supported for this element"),
		}
	}
}

impl std::error::Error for EventError {}
