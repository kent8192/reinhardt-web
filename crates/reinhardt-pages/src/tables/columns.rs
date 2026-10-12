//! Column type implementations
//!
//! This module provides various column types for different data rendering:
//! - `Column<T>`: Basic column for any type
//! - `LinkColumn`: Column with hyperlink
//! - `BooleanColumn`: Column for boolean values (checkmark/X)
//! - `DateTimeColumn`: Column for date/time formatting
//! - `EmailColumn`: Column for email addresses with mailto links
//! - `ChoiceColumn`: Column for choice fields
//! - `TemplateColumn`: Column with custom template
//! - `JSONColumn`: Column for JSON data
//! - `CheckBoxColumn`: Column with checkbox
//! - `URLColumn`: Column for URLs
//!
//! Every built-in column renders a `<td>` [`Page`] element, so cells render
//! identically on native and `wasm32` targets. Values whose type a column does
//! not support render as the `<td>-</td>` placeholder.

pub mod basic;
pub mod boolean;
pub mod checkbox;
pub mod choice;
pub mod datetime;
pub mod email;
pub mod json;
pub mod link;
pub mod template;
pub mod url;

// Re-exports
pub use basic::Column;
pub use boolean::BooleanColumn;
pub use checkbox::CheckBoxColumn;
pub use choice::ChoiceColumn;
pub use datetime::DateTimeColumn;
pub use email::EmailColumn;
pub use json::JSONColumn;
pub use link::LinkColumn;
pub use template::TemplateColumn;
pub use url::URLColumn;

use crate::component::{IntoPage, Page, PageElement};

/// Wraps cell content in the `<td>` element shared by all built-in columns.
fn cell(content: impl IntoPage) -> Page {
	Page::Element(PageElement::new("td").child(content))
}

/// Renders the placeholder cell for values of an unsupported type.
fn placeholder_cell() -> Page {
	cell("-")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tables::column::Column as ColumnTrait;
	use rstest::rstest;
	use std::any::Any;
	use std::collections::HashMap;

	#[rstest]
	fn table_columns_preserve_metadata_and_visibility_configuration() {
		// Arrange
		let basic = Column::<u32>::new("id", "Identifier")
			.orderable(false)
			.visible(false);
		let boolean = BooleanColumn::new("active", "Active")
			.orderable(false)
			.visible(false);
		let checkbox = CheckBoxColumn::new("selected", "Selected")
			.orderable(true)
			.visible(false);
		let choice = ChoiceColumn::new("status", "Status")
			.choices(HashMap::from([("draft".to_string(), "Draft".to_string())]))
			.orderable(false)
			.visible(false);
		let datetime = DateTimeColumn::new("created_at", "Created")
			.format("%Y-%m-%d")
			.orderable(false)
			.visible(false);

		// Act
		let metadata = [
			(
				basic.name(),
				basic.label(),
				basic.is_orderable(),
				basic.is_visible(),
			),
			(
				boolean.name(),
				boolean.label(),
				boolean.is_orderable(),
				boolean.is_visible(),
			),
			(
				checkbox.name(),
				checkbox.label(),
				checkbox.is_orderable(),
				checkbox.is_visible(),
			),
			(
				choice.name(),
				choice.label(),
				choice.is_orderable(),
				choice.is_visible(),
			),
			(
				datetime.name(),
				datetime.label(),
				datetime.is_orderable(),
				datetime.is_visible(),
			),
		];

		// Assert
		assert_eq!(
			metadata,
			[
				("id", "Identifier", false, false),
				("active", "Active", false, false),
				("selected", "Selected", true, false),
				("status", "Status", false, false),
				("created_at", "Created", false, false),
			]
		);
	}
	#[rstest]
	fn specialized_columns_expose_expected_defaults_and_custom_metadata() {
		// Arrange
		let boolean = BooleanColumn::with_icons("verified", "Verified", "yes", "no");
		let email = EmailColumn::new("email", "Email").visible(false);
		let json = JSONColumn::new("payload", "Payload").orderable(true);
		let link = LinkColumn::new("id", "Profile", "/profiles/{id}")
			.orderable(false)
			.visible(false);
		let link_with_text = LinkColumn::with_text("slug", "Article", "/articles/{slug}", "Read");
		let template = TemplateColumn::new("summary", "Summary")
			.orderable(true)
			.visible(false);
		let url = URLColumn::new("website", "Website")
			.orderable(false)
			.visible(false);

		// Act
		let metadata = [
			(
				boolean.name(),
				boolean.label(),
				boolean.is_orderable(),
				boolean.is_visible(),
			),
			(
				email.name(),
				email.label(),
				email.is_orderable(),
				email.is_visible(),
			),
			(
				json.name(),
				json.label(),
				json.is_orderable(),
				json.is_visible(),
			),
			(
				link.name(),
				link.label(),
				link.is_orderable(),
				link.is_visible(),
			),
			(
				link_with_text.name(),
				link_with_text.label(),
				link_with_text.is_orderable(),
				link_with_text.is_visible(),
			),
			(
				template.name(),
				template.label(),
				template.is_orderable(),
				template.is_visible(),
			),
			(
				url.name(),
				url.label(),
				url.is_orderable(),
				url.is_visible(),
			),
		];

		// Assert
		assert_eq!(
			metadata,
			[
				("verified", "Verified", true, true),
				("email", "Email", true, false),
				("payload", "Payload", true, true),
				("id", "Profile", false, false),
				("slug", "Article", true, true),
				("summary", "Summary", true, false),
				("website", "Website", false, false),
			]
		);
	}

	#[rstest]
	#[case::basic_display_value(
		Box::new(Column::<u32>::new("id", "ID")),
		Box::new(42_u32),
		"<td>42</td>"
	)]
	#[case::basic_escapes_html(
		Box::new(Column::<String>::new("name", "Name")),
		Box::new("<b>Ada</b>".to_string()),
		"<td>&lt;b&gt;Ada&lt;/b&gt;</td>"
	)]
	#[case::basic_unsupported_type(
		Box::new(Column::<u32>::new("id", "ID")),
		Box::new(42_i64),
		"<td>-</td>"
	)]
	#[case::boolean_true_default_icon(
		Box::new(BooleanColumn::new("active", "Active")),
		Box::new(true),
		"<td>\u{2713}</td>"
	)]
	#[case::boolean_false_default_icon(
		Box::new(BooleanColumn::new("active", "Active")),
		Box::new(false),
		"<td>\u{2717}</td>"
	)]
	#[case::boolean_custom_icon(
		Box::new(BooleanColumn::with_icons("verified", "Verified", "YES", "NO")),
		Box::new(false),
		"<td>NO</td>"
	)]
	#[case::boolean_unsupported_type(
		Box::new(BooleanColumn::new("active", "Active")),
		Box::new("true".to_string()),
		"<td>-</td>"
	)]
	#[case::checkbox_checked(
		Box::new(CheckBoxColumn::new("selected", "Selected")),
		Box::new(true),
		"<td><input type=\"checkbox\" checked=\"checked\" /></td>"
	)]
	#[case::checkbox_unchecked(
		Box::new(CheckBoxColumn::new("selected", "Selected")),
		Box::new(false),
		"<td><input type=\"checkbox\" /></td>"
	)]
	#[case::checkbox_unsupported_type(
		Box::new(CheckBoxColumn::new("selected", "Selected")),
		Box::new(1_u8),
		"<td><input type=\"checkbox\" /></td>"
	)]
	#[case::choice_mapped_label(
		Box::new(ChoiceColumn::new("status", "Status").choices(HashMap::from([(
			"draft".to_string(),
			"Draft".to_string()
		)]))),
		Box::new("draft".to_string()),
		"<td>Draft</td>"
	)]
	#[case::choice_unmapped_value(
		Box::new(ChoiceColumn::new("status", "Status")),
		Box::new("archived".to_string()),
		"<td>archived</td>"
	)]
	#[case::choice_unsupported_type(
		Box::new(ChoiceColumn::new("status", "Status")),
		Box::new(1_i32),
		"<td>-</td>"
	)]
	#[case::datetime_preformatted_string(
		Box::new(DateTimeColumn::new("created_at", "Created")),
		Box::new("2026-08-08 14:30:00".to_string()),
		"<td>2026-08-08 14:30:00</td>"
	)]
	#[case::datetime_unsupported_type(
		Box::new(DateTimeColumn::new("created_at", "Created")),
		Box::new(1_i64),
		"<td>-</td>"
	)]
	#[case::email_mailto_link(
		Box::new(EmailColumn::new("email", "Email")),
		Box::new("ada@example.com".to_string()),
		"<td><a href=\"mailto:ada@example.com\">ada@example.com</a></td>"
	)]
	#[case::email_unsupported_type(
		Box::new(EmailColumn::new("email", "Email")),
		Box::new(1_i32),
		"<td>-</td>"
	)]
	#[case::json_value_pretty_printed(
		Box::new(JSONColumn::new("payload", "Payload")),
		Box::new(serde_json::json!({ "id": 1 })),
		"<td><pre>{\n  &quot;id&quot;: 1\n}</pre></td>"
	)]
	#[case::json_raw_string(
		Box::new(JSONColumn::new("payload", "Payload")),
		Box::new("{\"id\":1}".to_string()),
		"<td>{&quot;id&quot;:1}</td>"
	)]
	#[case::json_unsupported_type(
		Box::new(JSONColumn::new("payload", "Payload")),
		Box::new(1_i32),
		"<td>-</td>"
	)]
	#[case::link_string_value(
		Box::new(LinkColumn::new("id", "Profile", "/profiles/{id}")),
		Box::new("42".to_string()),
		"<td><a href=\"/profiles/42\">42</a></td>"
	)]
	#[case::link_static_str_value(
		Box::new(LinkColumn::new("slug", "Article", "/articles/{slug}")),
		Box::new("coverage"),
		"<td><a href=\"/articles/coverage\">coverage</a></td>"
	)]
	#[case::link_i32_value(
		Box::new(LinkColumn::new("id", "Profile", "/profiles/{id}")),
		Box::new(7_i32),
		"<td><a href=\"/profiles/7\">7</a></td>"
	)]
	#[case::link_i64_value(
		Box::new(LinkColumn::new("id", "Profile", "/profiles/{id}")),
		Box::new(9_i64),
		"<td><a href=\"/profiles/9\">9</a></td>"
	)]
	#[case::link_text_override(
		Box::new(LinkColumn::with_text("slug", "Article", "/articles/{slug}", "Read")),
		Box::new("coverage".to_string()),
		"<td><a href=\"/articles/coverage\">Read</a></td>"
	)]
	#[case::link_unsupported_type(
		Box::new(LinkColumn::new("id", "Profile", "/profiles/{id}")),
		Box::new(1.5_f64),
		"<td><a href=\"/profiles/\"></a></td>"
	)]
	#[case::template_string(
		Box::new(TemplateColumn::new("summary", "Summary")),
		Box::new("Quarterly report".to_string()),
		"<td>Quarterly report</td>"
	)]
	#[case::template_static_str(
		Box::new(TemplateColumn::new("summary", "Summary")),
		Box::new("Weekly digest"),
		"<td>Weekly digest</td>"
	)]
	#[case::template_unsupported_type(
		Box::new(TemplateColumn::new("summary", "Summary")),
		Box::new(1_i32),
		"<td>-</td>"
	)]
	#[case::url_link(
		Box::new(URLColumn::new("website", "Website")),
		Box::new("https://example.com/docs".to_string()),
		"<td><a href=\"https://example.com/docs\">https://example.com/docs</a></td>"
	)]
	#[case::url_unsafe_scheme_drops_href(
		Box::new(URLColumn::new("website", "Website")),
		Box::new("javascript:alert(1)".to_string()),
		"<td><a>javascript:alert(1)</a></td>"
	)]
	#[case::url_unsupported_type(
		Box::new(URLColumn::new("website", "Website")),
		Box::new(1_i32),
		"<td>-</td>"
	)]
	fn built_in_columns_render_cells_to_html_on_native_targets(
		#[case] column: Box<dyn ColumnTrait>,
		#[case] value: Box<dyn Any>,
		#[case] expected: &str,
	) {
		// Act
		let html = column.render(value.as_ref()).render_to_string();

		// Assert
		assert_eq!(html, expected);
	}

	#[cfg(feature = "chrono")]
	#[rstest]
	fn datetime_column_formats_chrono_values() {
		// Arrange
		let column = DateTimeColumn::new("created_at", "Created").format("%Y/%m/%d %H:%M");
		let naive =
			chrono::NaiveDateTime::parse_from_str("2026-08-08 14:30:00", "%Y-%m-%d %H:%M:%S")
				.expect("fixture timestamp should parse");
		let utc = naive.and_utc();

		// Act
		let rendered = (
			column.render(&naive).render_to_string(),
			column.render(&utc).render_to_string(),
		);

		// Assert
		assert_eq!(
			rendered,
			(
				"<td>2026/08/08 14:30</td>".to_string(),
				"<td>2026/08/08 14:30</td>".to_string(),
			)
		);
	}
}
