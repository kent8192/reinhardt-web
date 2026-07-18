//! Stable identifiers for route-level data loaders.

/// Identifies a loader independently of the route URL that currently uses it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RouteLoaderId(&'static str);

impl RouteLoaderId {
	/// Creates a loader identifier from a stable string.
	pub const fn new(value: &'static str) -> Self {
		Self(value)
	}

	/// Returns the stable string representation.
	pub const fn as_str(self) -> &'static str {
		self.0
	}
}
