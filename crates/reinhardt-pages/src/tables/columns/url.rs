//! URL column type implementation

use super::{cell, placeholder_cell};
use crate::component::{Page, PageElement};
use crate::tables::column::Column as ColumnTrait;
use std::any::Any;

/// Column for URLs
///
/// This column type renders `String` URLs as clickable links. The `href`
/// passes through the [`Page`] URL sanitizer, so URLs with an unsafe scheme
/// such as `javascript:` render as link text without an `href` attribute.
pub struct URLColumn {
	name: String,
	label: String,
	orderable: bool,
	visible: bool,
}

impl URLColumn {
	/// Creates a new URL column
	pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			orderable: true,
			visible: true,
		}
	}

	/// Sets whether this column is orderable
	pub fn orderable(mut self, orderable: bool) -> Self {
		self.orderable = orderable;
		self
	}

	/// Sets whether this column is visible
	pub fn visible(mut self, visible: bool) -> Self {
		self.visible = visible;
		self
	}
}

impl ColumnTrait for URLColumn {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> &str {
		&self.label
	}

	fn render(&self, value: &dyn Any) -> Page {
		match value.downcast_ref::<String>() {
			Some(url) => cell(PageElement::new("a").attr("href", url.clone()).child(url)),
			None => placeholder_cell(),
		}
	}

	fn is_orderable(&self) -> bool {
		self.orderable
	}

	fn is_visible(&self) -> bool {
		self.visible
	}
}
