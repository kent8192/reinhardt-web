//! Basic column type implementation

use super::{cell, placeholder_cell};
use crate::component::Page;
use crate::tables::column::Column as ColumnTrait;
use std::any::Any;
use std::marker::PhantomData;

/// Basic column for any type that implements `Display`
///
/// This is the most basic column type that renders values as-is using their
/// `Display` implementation.
///
/// Values of any other type render as the `<td>-</td>` placeholder.
///
/// # Example
///
/// ```
/// use reinhardt_pages::tables::{ColumnTrait, columns::Column};
///
/// let column = Column::<String>::new("name", "Name");
///
/// assert_eq!(
///     column.render(&String::from("alice")).render_to_string(),
///     "<td>alice</td>"
/// );
/// ```
pub struct Column<T> {
	name: String,
	label: String,
	orderable: bool,
	visible: bool,
	_phantom: PhantomData<T>,
}

impl<T> Column<T> {
	/// Creates a new basic column
	///
	/// # Arguments
	///
	/// * `name` - The internal field name
	/// * `label` - The display label
	pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			orderable: true,
			visible: true,
			_phantom: PhantomData,
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

impl<T: std::fmt::Display + 'static> ColumnTrait for Column<T> {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> &str {
		&self.label
	}

	fn render(&self, value: &dyn Any) -> Page {
		match value.downcast_ref::<T>() {
			Some(val) => cell(val.to_string()),
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
