//! Native rendering tests for table columns through the public API

use reinhardt_pages::component::{Page, PageElement};
use reinhardt_pages::tables::{ColumnTrait, columns::Column};
use rstest::rstest;

#[rstest]
fn basic_column_renders_cell_without_browser_globals() {
	// Arrange
	let column = Column::<String>::new("name", "Name");
	let value = String::from("alice");

	// Act
	let html = column.render(&value).render_to_string();

	// Assert
	assert_eq!(html, "<td>alice</td>");
}

#[rstest]
fn rendered_cells_compose_into_page_elements() {
	// Arrange
	let name = Column::<String>::new("name", "Name");
	let age = Column::<u32>::new("age", "Age");

	// Act
	let row = PageElement::new("tr")
		.child(name.render(&String::from("alice")))
		.child(age.render(&36_u32));

	// Assert
	assert_eq!(
		Page::Element(row).render_to_string(),
		"<tr><td>alice</td><td>36</td></tr>"
	);
}
