//! Shared Todo domain types.

use serde::{Deserialize, Serialize};

/// A Todo item displayed by the example application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
	/// Stable item identifier assigned by the server function store.
	pub id: u64,
	/// User-entered item title.
	pub title: String,
	/// Completion state.
	pub completed: bool,
}

impl TodoItem {
	/// Creates a new incomplete Todo item.
	pub fn new(id: u64, title: impl Into<String>) -> Self {
		Self {
			id,
			title: title.into(),
			completed: false,
		}
	}
}

/// Filter applied to the Todo list route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoFilter {
	/// Show every Todo item.
	All,
	/// Show incomplete Todo items.
	Active,
	/// Show completed Todo items.
	Completed,
}

impl TodoFilter {
	/// Returns the route path for this filter.
	pub fn path(self) -> &'static str {
		match self {
			Self::All => "/",
			Self::Active => "/active/",
			Self::Completed => "/completed/",
		}
	}

	/// Returns the display label for this filter.
	pub fn label(self) -> &'static str {
		match self {
			Self::All => "All",
			Self::Active => "Active",
			Self::Completed => "Completed",
		}
	}

	/// Returns whether a Todo item belongs in this filter.
	pub fn matches(self, todo: &TodoItem) -> bool {
		match self {
			Self::All => true,
			Self::Active => !todo.completed,
			Self::Completed => todo.completed,
		}
	}
}
