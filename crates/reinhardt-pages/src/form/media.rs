//! Media assets for form widgets
//!
//! This module provides CSS and JavaScript asset management for form widgets.
//! It allows forms to specify their required CSS and JavaScript resources.

/// Media assets for form widgets
///
/// Contains CSS and JavaScript resources required by form widgets.
#[derive(Debug, Clone, Default)]
pub struct Media {
	/// CSS resources
	css: Vec<String>,
	/// JavaScript resources
	js: Vec<String>,
}

impl Media {
	/// Create a new empty Media instance
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a CSS resource
	pub fn add_css(&mut self, css: impl Into<String>) {
		self.css.push(css.into());
	}

	/// Add a JavaScript resource
	pub fn add_js(&mut self, js: impl Into<String>) {
		self.js.push(js.into());
	}

	/// Get CSS resources
	pub fn css(&self) -> &[String] {
		&self.css
	}

	/// Get JavaScript resources
	pub fn js(&self) -> &[String] {
		&self.js
	}

	/// Check if media is empty
	pub fn is_empty(&self) -> bool {
		self.css.is_empty() && self.js.is_empty()
	}

	/// Merge another Media instance into this one
	pub fn extend(&mut self, other: &Media) {
		self.css.extend(other.css.iter().cloned());
		self.js.extend(other.js.iter().cloned());
	}

	/// Render CSS link tags
	pub fn render_css(&self) -> String {
		self.css
			.iter()
			.map(|path| format!(r#"<link rel="stylesheet" href="{}">"#, path))
			.collect::<Vec<_>>()
			.join("\n")
	}

	/// Render JavaScript script tags
	pub fn render_js(&self) -> String {
		self.js
			.iter()
			.map(|path| format!(r#"<script src="{}"></script>"#, path))
			.collect::<Vec<_>>()
			.join("\n")
	}
}

/// Trait for widgets that define their own media
pub trait MediaDefiningWidget {
	/// Get the media required by this widget
	fn media(&self) -> Media;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_media_creation() {
		let media = Media::new();
		assert!(media.is_empty());
	}

	#[rstest]
	fn test_media_add_resources() {
		let mut media = Media::new();
		media.add_css("/static/css/forms.css");
		media.add_js("/static/js/forms.js");

		assert!(!media.is_empty());
		assert_eq!(media.css(), &["/static/css/forms.css".to_string()]);
		assert_eq!(media.js(), &["/static/js/forms.js".to_string()]);
	}

	#[rstest]
	fn test_media_render_css() {
		let mut media = Media::new();
		media.add_css("/static/css/forms.css");
		media.add_css("/static/css/widgets.css");

		let rendered = media.render_css();
		assert!(rendered.contains(r#"<link rel="stylesheet" href="/static/css/forms.css">"#));
		assert!(rendered.contains(r#"<link rel="stylesheet" href="/static/css/widgets.css">"#));
	}

	#[rstest]
	fn test_media_render_js() {
		let mut media = Media::new();
		media.add_js("/static/js/forms.js");

		let rendered = media.render_js();
		assert_eq!(rendered, r#"<script src="/static/js/forms.js"></script>"#);
	}

	#[rstest]
	fn test_media_extend() {
		let mut media1 = Media::new();
		media1.add_css("/static/css/base.css");

		let mut media2 = Media::new();
		media2.add_css("/static/css/extra.css");
		media2.add_js("/static/js/extra.js");

		media1.extend(&media2);

		assert_eq!(media1.css().len(), 2);
		assert_eq!(media1.js().len(), 1);
	}
}
