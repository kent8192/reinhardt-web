//! Boolean column type implementation

use super::{cell, placeholder_cell};
use crate::component::Page;
use crate::tables::column::Column as ColumnTrait;
use std::any::Any;

/// Column for boolean values
///
/// This column type renders boolean values as checkmarks (✓) for `true`
/// and X marks (✗) for `false`. Custom icons can be provided.
///
/// # Example
///
/// ```
/// use reinhardt_pages::tables::{BooleanColumn, ColumnTrait};
///
/// let column = BooleanColumn::new("is_active", "Active");
///
/// assert_eq!(column.render(&true).render_to_string(), "<td>✓</td>");
/// ```
pub struct BooleanColumn {
	name: String,
	label: String,
	true_icon: String,
	false_icon: String,
	orderable: bool,
	visible: bool,
}

impl BooleanColumn {
	/// Creates a new boolean column with default icons
	///
	/// # Arguments
	///
	/// * `name` - The internal field name
	/// * `label` - The display label
	pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			true_icon: "✓".to_string(),
			false_icon: "✗".to_string(),
			orderable: true,
			visible: true,
		}
	}

	/// Creates a new boolean column with custom icons
	///
	/// # Arguments
	///
	/// * `name` - The internal field name
	/// * `label` - The display label
	/// * `true_icon` - Icon to display for `true` values
	/// * `false_icon` - Icon to display for `false` values
	pub fn with_icons(
		name: impl Into<String>,
		label: impl Into<String>,
		true_icon: impl Into<String>,
		false_icon: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			true_icon: true_icon.into(),
			false_icon: false_icon.into(),
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

impl ColumnTrait for BooleanColumn {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> &str {
		&self.label
	}

	fn render(&self, value: &dyn Any) -> Page {
		match value.downcast_ref::<bool>() {
			Some(true) => cell(&self.true_icon),
			Some(false) => cell(&self.false_icon),
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
