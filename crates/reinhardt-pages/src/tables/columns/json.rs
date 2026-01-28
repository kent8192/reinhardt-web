//! JSON column type implementation

use crate::builder::html::td;
use crate::dom::Element;
use crate::tables::column::Column as ColumnTrait;
use std::any::Any;

/// Column for JSON data
///
/// This column type renders JSON data with optional formatting.
pub struct JSONColumn {
	name: String,
	label: String,
	orderable: bool,
	visible: bool,
}

impl JSONColumn {
	/// Creates a new JSON column
	pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			orderable: false,
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

impl ColumnTrait for JSONColumn {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> &str {
		&self.label
	}

	fn render(&self, _value: &dyn Any) -> Element {
		// TODO: Implement JSON rendering
		td().text("json").build()
	}

	fn is_orderable(&self) -> bool {
		self.orderable
	}

	fn is_visible(&self) -> bool {
		self.visible
	}
}
