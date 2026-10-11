//! Table trait and basic implementation

use super::column::Column;
use super::sorting::SortDirection;
use crate::component::Page;

/// Represents a table with rows and columns
///
/// This trait defines the core functionality for tables including
/// data access, rendering, sorting, and pagination.
///
/// # Rendering
///
/// [`Table::render`] returns a target-neutral [`Page`], so a table renders the
/// same way during SSR, in native tests, and in the browser. Because `Table`
/// is implemented by application types rather than a concrete framework type,
/// the [`Page`] returned by `render` is the adapter into the view tree: pass it
/// to `page!` as a `Page` parameter, add it as a child of a
/// [`PageElement`](crate::component::PageElement), or call
/// [`Page::render_to_string`] for HTML. See the [module documentation](super)
/// for a complete example.
pub trait Table {
	/// The type of data rows in this table
	type Row;

	/// Returns all rows in the table
	fn rows(&self) -> Vec<&Self::Row>;

	/// Returns all columns in the table
	fn columns(&self) -> Vec<&dyn Column>;

	/// Renders the table as a [`Page`].
	///
	/// Implementations typically build a `<table>` element whose cells come
	/// from [`Column::render`].
	fn render(&self) -> Page;

	/// Handles sorting by the specified field and direction
	fn handle_sort(&mut self, field: &str, direction: SortDirection);

	/// Handles pagination by setting the current page
	fn handle_pagination(&mut self, page: usize);

	/// Returns the total number of rows
	fn total_rows(&self) -> usize {
		self.rows().len()
	}

	/// Returns the current page number (1-indexed)
	fn current_page(&self) -> usize {
		1
	}

	/// Returns the number of rows per page
	fn per_page(&self) -> usize {
		10
	}

	/// Returns whether the table supports sorting
	fn is_sortable(&self) -> bool {
		true
	}

	/// Returns whether the table supports pagination
	fn is_paginated(&self) -> bool {
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::component::PageElement;
	use crate::tables::columns::{BooleanColumn, Column as BasicColumn};
	use rstest::*;

	struct User {
		id: u32,
		name: String,
		active: bool,
	}

	struct ExampleTable {
		rows: Vec<User>,
		id: BasicColumn<u32>,
		name: BasicColumn<String>,
		active: BooleanColumn,
	}

	impl Table for ExampleTable {
		type Row = User;

		fn rows(&self) -> Vec<&Self::Row> {
			self.rows.iter().collect()
		}

		fn columns(&self) -> Vec<&dyn Column> {
			vec![&self.id, &self.name, &self.active]
		}

		fn render(&self) -> Page {
			let header = PageElement::new("tr").children(
				self.columns()
					.into_iter()
					.map(|column| PageElement::new("th").child(column.label().to_owned())),
			);
			let body = self.rows().into_iter().map(|user| {
				PageElement::new("tr")
					.child(self.id.render(&user.id))
					.child(self.name.render(&user.name))
					.child(self.active.render(&user.active))
			});
			Page::Element(
				PageElement::new("table")
					.child(PageElement::new("thead").child(header))
					.child(PageElement::new("tbody").children(body)),
			)
		}

		fn handle_sort(&mut self, _field: &str, _direction: SortDirection) {}

		fn handle_pagination(&mut self, _page: usize) {}
	}

	#[fixture]
	fn table() -> ExampleTable {
		ExampleTable {
			rows: vec![
				User {
					id: 1,
					name: "Ada".to_string(),
					active: true,
				},
				User {
					id: 2,
					name: "<Grace>".to_string(),
					active: false,
				},
			],
			id: BasicColumn::new("id", "ID"),
			name: BasicColumn::new("name", "Name"),
			active: BooleanColumn::new("active", "Active"),
		}
	}

	const EXPECTED_TABLE_HTML: &str = concat!(
		"<table>",
		"<thead><tr><th>ID</th><th>Name</th><th>Active</th></tr></thead>",
		"<tbody>",
		"<tr><td>1</td><td>Ada</td><td>\u{2713}</td></tr>",
		"<tr><td>2</td><td>&lt;Grace&gt;</td><td>\u{2717}</td></tr>",
		"</tbody>",
		"</table>",
	);

	#[rstest]
	fn sorting_and_table_defaults_match_public_contract(table: ExampleTable) {
		// Act
		let defaults = (
			table.total_rows(),
			table.current_page(),
			table.per_page(),
			table.is_sortable(),
			table.is_paginated(),
			table.columns().len(),
		);

		// Assert
		assert_eq!(defaults, (2, 1, 10, true, false, 3));
	}

	#[rstest]
	fn table_renders_header_and_rows_to_html_on_native_targets(table: ExampleTable) {
		// Act
		let html = table.render().render_to_string();

		// Assert
		assert_eq!(html, EXPECTED_TABLE_HTML);
	}

	#[rstest]
	fn rendered_table_embeds_inside_page_macro(table: ExampleTable) {
		// Arrange
		let rendered = table.render();

		// Act
		let view = crate::page!(|rendered: Page| {
			section {
				{ rendered }
			}
		})(rendered);

		// Assert
		assert_eq!(
			view.render_to_string(),
			format!("<section>{EXPECTED_TABLE_HTML}</section>")
		);
	}
}
