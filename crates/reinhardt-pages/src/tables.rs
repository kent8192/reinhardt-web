//! Table utilities for reinhardt-pages (django-tables2 equivalent)
//!
//! This module provides table rendering with various column types,
//! sorting, pagination, filtering, and export functionality.
//!
//! # Features
//!
//! - Multiple column types: `Column<T>`, `LinkColumn`, `BooleanColumn`,
//!   `CheckBoxColumn`, `DateTimeColumn`, `EmailColumn`, `ChoiceColumn`,
//!   `TemplateColumn`, `JSONColumn`, `URLColumn`
//! - Sorting with `SortDirection` and `Sortable` trait
//! - Pagination with `Pagination` struct
//! - Filtering with `Filterable` trait
//! - Export to CSV and JSON
//!
//! # Rendering
//!
//! [`ColumnTrait::render`] and [`Table::render`] return a target-neutral
//! [`Page`](crate::component::Page) built from
//! [`PageElement`](crate::component::PageElement)s. Rendering does not touch
//! the browser DOM, so tables have behavioral (P2) parity across targets:
//!
//! - SSR and native tests serialize a table with
//!   [`Page::render_to_string`](crate::component::Page::render_to_string).
//! - Components return the rendered `Page` directly or embed it in `page!` as
//!   a `Page` parameter.
//! - In the browser, the `Page` is mounted like any other view.
//!
//! # Example
//!
//! ```
//! use reinhardt_pages::component::{Page, PageElement};
//! use reinhardt_pages::page;
//! use reinhardt_pages::tables::{
//!     BooleanColumn, ColumnTrait, LinkColumn, SortDirection, Table, columns::Column,
//! };
//!
//! struct User {
//!     id: i64,
//!     name: String,
//!     active: bool,
//! }
//!
//! struct UserTable {
//!     users: Vec<User>,
//!     name: Column<String>,
//!     profile: LinkColumn,
//!     active: BooleanColumn,
//! }
//!
//! impl Table for UserTable {
//!     type Row = User;
//!
//!     fn rows(&self) -> Vec<&User> {
//!         self.users.iter().collect()
//!     }
//!
//!     fn columns(&self) -> Vec<&dyn ColumnTrait> {
//!         vec![&self.name, &self.profile, &self.active]
//!     }
//!
//!     fn render(&self) -> Page {
//!         let header = PageElement::new("tr").children(
//!             self.columns()
//!                 .into_iter()
//!                 .map(|column| PageElement::new("th").child(column.label().to_owned())),
//!         );
//!         let rows = self.rows().into_iter().map(|user| {
//!             PageElement::new("tr")
//!                 .child(self.name.render(&user.name))
//!                 .child(self.profile.render(&user.id))
//!                 .child(self.active.render(&user.active))
//!         });
//!         Page::Element(
//!             PageElement::new("table")
//!                 .child(PageElement::new("thead").child(header))
//!                 .child(PageElement::new("tbody").children(rows)),
//!         )
//!     }
//!
//!     fn handle_sort(&mut self, _field: &str, _direction: SortDirection) {}
//!
//!     fn handle_pagination(&mut self, _page: usize) {}
//! }
//!
//! let table = UserTable {
//!     users: vec![User { id: 7, name: "Ada".to_owned(), active: true }],
//!     name: Column::new("name", "Name"),
//!     profile: LinkColumn::with_text("id", "Profile", "/users/{id}", "View"),
//!     active: BooleanColumn::with_icons("active", "Active", "yes", "no"),
//! };
//!
//! let view = page!(|table: Page| {
//!     section {
//!         { table }
//!     }
//! })(table.render());
//!
//! assert_eq!(
//!     view.render_to_string(),
//!     concat!(
//!         "<section><table>",
//!         "<thead><tr><th>Name</th><th>Profile</th><th>Active</th></tr></thead>",
//!         "<tbody><tr><td>Ada</td>",
//!         "<td><a href=\"/users/7\">View</a></td><td>yes</td>",
//!         "</tr></tbody>",
//!         "</table></section>",
//!     )
//! );
//! ```

pub mod column;
pub mod columns;
pub mod export;
pub mod filtering;
pub mod pagination;
pub mod sorting;
pub mod table;

// Re-exports for convenience
pub use column::Column as ColumnTrait;
pub use columns::*;
pub use export::{ExportFormat, Exportable};
pub use filtering::Filterable;
pub use pagination::Pagination;
pub use sorting::{SortDirection, Sortable};
pub use table::Table;
