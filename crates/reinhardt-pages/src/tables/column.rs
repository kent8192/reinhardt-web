//! Column trait and basic implementation

use crate::component::Page;
use std::any::Any;

/// Represents a table column
///
/// This trait defines the core functionality for table columns including
/// rendering, ordering, and metadata access.
///
/// Columns render to a target-neutral [`Page`], so the same cell can be
/// serialized during SSR, asserted in native tests, or mounted in the browser.
///
/// # Example
///
/// ```
/// use reinhardt_pages::tables::{ColumnTrait, columns::Column};
///
/// let column = Column::<String>::new("name", "Name");
/// let cell = column.render(&String::from("alice"));
///
/// assert_eq!(cell.render_to_string(), "<td>alice</td>");
/// ```
pub trait Column {
	/// Returns the column name (internal field name)
	fn name(&self) -> &str;

	/// Returns the column display label
	fn label(&self) -> &str;

	/// Renders the cell for `value` as a [`Page`].
	///
	/// Built-in columns return a `<td>` element and fall back to a `<td>-</td>`
	/// placeholder when `value` has a type the column does not support. Call
	/// [`Page::render_to_string`] for HTML, or pass the page to `page!` or any
	/// other [`IntoPage`](crate::component::IntoPage) consumer to mount it.
	fn render(&self, value: &dyn Any) -> Page;

	/// Returns whether this column is orderable (sortable)
	fn is_orderable(&self) -> bool {
		true
	}

	/// Returns whether this column is visible
	fn is_visible(&self) -> bool {
		true
	}

	/// Returns custom HTML attributes for the column header
	fn header_attrs(&self) -> Vec<(&str, &str)> {
		vec![]
	}

	/// Returns custom HTML attributes for the column cell
	fn cell_attrs(&self) -> Vec<(&str, &str)> {
		vec![]
	}
}
