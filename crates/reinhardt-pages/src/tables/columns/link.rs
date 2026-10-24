//! Link column type implementation

use super::cell;
use crate::component::{Page, PageElement};
use crate::tables::column::Column as ColumnTrait;
use std::any::Any;

/// Column that renders values as hyperlinks
///
/// This column type renders values as HTML anchor tags with customizable URLs.
/// The URL pattern can include placeholders like `{id}` that will be replaced
/// with actual values. `String`, `&'static str`, `i32`, and `i64` values are
/// supported; other types substitute an empty string.
///
/// The resulting URL passes through the [`Page`] URL sanitizer, which omits
/// the `href` attribute when the URL starts with `../` or uses a scheme other
/// than `http`, `https`, `mailto`, `ftp`, `ftps`, or `tel`.
///
/// # Example
///
/// ```
/// use reinhardt_pages::tables::{ColumnTrait, LinkColumn};
///
/// // Simple link
/// let column = LinkColumn::new("id", "Profile", "/users/{id}");
/// assert_eq!(
///     column.render(&String::from("42")).render_to_string(),
///     r#"<td><a href="/users/42">42</a></td>"#
/// );
///
/// // With custom text
/// let column = LinkColumn::with_text("id", "Profile", "/users/{id}", "View Profile");
/// assert_eq!(
///     column.render(&42_i64).render_to_string(),
///     r#"<td><a href="/users/42">View Profile</a></td>"#
/// );
/// ```
pub struct LinkColumn {
	name: String,
	label: String,
	url_pattern: String,
	text_override: Option<String>,
	orderable: bool,
	visible: bool,
}

impl LinkColumn {
	/// Creates a new link column
	///
	/// # Arguments
	///
	/// * `name` - The internal field name
	/// * `label` - The display label
	/// * `url_pattern` - URL pattern with placeholders (e.g., "/users/{id}")
	pub fn new(
		name: impl Into<String>,
		label: impl Into<String>,
		url_pattern: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			url_pattern: url_pattern.into(),
			text_override: None,
			orderable: true,
			visible: true,
		}
	}

	/// Creates a new link column with custom link text
	///
	/// # Arguments
	///
	/// * `name` - The internal field name
	/// * `label` - The display label
	/// * `url_pattern` - URL pattern with placeholders (e.g., "/users/{id}")
	/// * `text` - Custom text to display instead of the value
	pub fn with_text(
		name: impl Into<String>,
		label: impl Into<String>,
		url_pattern: impl Into<String>,
		text: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			url_pattern: url_pattern.into(),
			text_override: Some(text.into()),
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

impl ColumnTrait for LinkColumn {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> &str {
		&self.label
	}

	fn render(&self, value: &dyn Any) -> Page {
		let value_str = if let Some(s) = value.downcast_ref::<String>() {
			s.clone()
		} else if let Some(s) = value.downcast_ref::<&str>() {
			(*s).to_string()
		} else if let Some(n) = value.downcast_ref::<i32>() {
			n.to_string()
		} else if let Some(n) = value.downcast_ref::<i64>() {
			n.to_string()
		} else {
			String::new()
		};

		// Replace {field_name} placeholders in URL pattern
		let url = self
			.url_pattern
			.replace(&format!("{{{}}}", self.name), &value_str);

		let text = self.text_override.clone().unwrap_or(value_str);
		cell(PageElement::new("a").attr("href", url).child(text))
	}

	fn is_orderable(&self) -> bool {
		self.orderable
	}

	fn is_visible(&self) -> bool {
		self.visible
	}
}
