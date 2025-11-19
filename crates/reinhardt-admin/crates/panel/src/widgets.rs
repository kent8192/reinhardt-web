//! Custom field widgets for admin forms
//!
//! This module provides customizable widgets for rendering form fields
//! with enhanced functionality and styling.
//!
//! # Examples
//!
//! ## Rich Text Editor
//!
//! ```
//! use reinhardt_panel::widgets::{RichTextEditorConfig, EditorType, Widget, WidgetType};
//!
//! // Create a TinyMCE editor
//! let config = RichTextEditorConfig::new(EditorType::TinyMCE)
//!     .with_toolbar("bold italic | link image")
//!     .with_max_length(5000);
//!
//! let widget = Widget::new(WidgetType::RichTextEditorWidget { config });
//! ```
//!
//! ## Image Upload
//!
//! ```
//! use reinhardt_panel::widgets::{ImageUploadConfig, ImageFormat, Widget, WidgetType};
//!
//! // Create an image upload widget
//! let config = ImageUploadConfig::new()
//!     .with_preview_size(400, 300)
//!     .with_formats(vec![ImageFormat::Jpeg, ImageFormat::Png])
//!     .with_max_size(10 * 1024 * 1024); // 10MB
//!
//! let widget = Widget::new(WidgetType::ImageUploadWidget { config });
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rich text editor backend type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EditorType {
	/// TinyMCE editor (popular WYSIWYG editor)
	TinyMCE,
	/// CKEditor (feature-rich editor)
	CKEditor,
	/// Quill (modern WYSIWYG editor)
	Quill,
	/// Simple textarea with basic formatting
	Simple,
}

/// Configuration for rich text editor widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextEditorConfig {
	/// Editor type to use
	pub editor_type: EditorType,
	/// Toolbar configuration (comma-separated list)
	pub toolbar: String,
	/// Maximum length of content
	pub max_length: Option<usize>,
	/// Allowed HTML tags (e.g., "p,a,strong,em")
	pub allowed_tags: Option<String>,
	/// Enable file upload support
	pub file_upload_enabled: bool,
	/// Custom CSS classes
	pub css_class: String,
}

impl Default for RichTextEditorConfig {
	fn default() -> Self {
		Self {
			editor_type: EditorType::TinyMCE,
			toolbar: "bold italic underline | link image | bullist numlist".into(),
			max_length: None,
			allowed_tags: Some("p,a,strong,em,ul,ol,li,br,img".into()),
			file_upload_enabled: false,
			css_class: "rich-text-editor".into(),
		}
	}
}

impl RichTextEditorConfig {
	/// Create a new rich text editor configuration
	pub fn new(editor_type: EditorType) -> Self {
		Self {
			editor_type,
			..Default::default()
		}
	}

	/// Set toolbar configuration
	pub fn with_toolbar(mut self, toolbar: impl Into<String>) -> Self {
		self.toolbar = toolbar.into();
		self
	}

	/// Set maximum content length
	pub fn with_max_length(mut self, max_length: usize) -> Self {
		self.max_length = Some(max_length);
		self
	}

	/// Set allowed HTML tags
	pub fn with_allowed_tags(mut self, tags: impl Into<String>) -> Self {
		self.allowed_tags = Some(tags.into());
		self
	}

	/// Enable or disable file upload
	pub fn with_file_upload(mut self, enabled: bool) -> Self {
		self.file_upload_enabled = enabled;
		self
	}

	/// Add custom CSS class
	pub fn with_css_class(mut self, css_class: impl Into<String>) -> Self {
		self.css_class = css_class.into();
		self
	}
}

/// Allowed image formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageFormat {
	/// JPEG format
	Jpeg,
	/// PNG format
	Png,
	/// GIF format
	Gif,
	/// WebP format
	WebP,
}

impl ImageFormat {
	/// Get MIME type for the format
	pub fn mime_type(&self) -> &str {
		match self {
			ImageFormat::Jpeg => "image/jpeg",
			ImageFormat::Png => "image/png",
			ImageFormat::Gif => "image/gif",
			ImageFormat::WebP => "image/webp",
		}
	}

	/// Get file extension for the format
	pub fn extension(&self) -> &str {
		match self {
			ImageFormat::Jpeg => "jpg",
			ImageFormat::Png => "png",
			ImageFormat::Gif => "gif",
			ImageFormat::WebP => "webp",
		}
	}
}

/// Configuration for image upload widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUploadConfig {
	/// Preview width in pixels
	pub preview_width: u32,
	/// Preview height in pixels
	pub preview_height: u32,
	/// Allowed image formats
	pub allowed_formats: Vec<ImageFormat>,
	/// Maximum file size in bytes
	pub max_file_size: usize,
	/// Enable thumbnail generation
	pub generate_thumbnail: bool,
	/// Thumbnail width in pixels
	pub thumbnail_width: Option<u32>,
	/// Thumbnail height in pixels
	pub thumbnail_height: Option<u32>,
	/// Enable crop functionality
	pub enable_crop: bool,
	/// Enable resize functionality
	pub enable_resize: bool,
	/// Enable drag & drop
	pub enable_drag_drop: bool,
	/// Custom CSS classes
	pub css_class: String,
}

impl Default for ImageUploadConfig {
	fn default() -> Self {
		Self {
			preview_width: 300,
			preview_height: 300,
			allowed_formats: vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Gif],
			max_file_size: 5 * 1024 * 1024, // 5MB
			generate_thumbnail: true,
			thumbnail_width: Some(150),
			thumbnail_height: Some(150),
			enable_crop: true,
			enable_resize: true,
			enable_drag_drop: true,
			css_class: "image-upload-widget".into(),
		}
	}
}

impl ImageUploadConfig {
	/// Create a new image upload configuration
	pub fn new() -> Self {
		Self::default()
	}

	/// Set preview dimensions
	pub fn with_preview_size(mut self, width: u32, height: u32) -> Self {
		self.preview_width = width;
		self.preview_height = height;
		self
	}

	/// Set allowed image formats
	pub fn with_formats(mut self, formats: Vec<ImageFormat>) -> Self {
		self.allowed_formats = formats;
		self
	}

	/// Set maximum file size in bytes
	pub fn with_max_size(mut self, size: usize) -> Self {
		self.max_file_size = size;
		self
	}

	/// Enable or disable thumbnail generation
	pub fn with_thumbnail(mut self, enabled: bool) -> Self {
		self.generate_thumbnail = enabled;
		self
	}

	/// Set thumbnail dimensions
	pub fn with_thumbnail_size(mut self, width: u32, height: u32) -> Self {
		self.thumbnail_width = Some(width);
		self.thumbnail_height = Some(height);
		self
	}

	/// Enable or disable crop functionality
	pub fn with_crop(mut self, enabled: bool) -> Self {
		self.enable_crop = enabled;
		self
	}

	/// Enable or disable resize functionality
	pub fn with_resize(mut self, enabled: bool) -> Self {
		self.enable_resize = enabled;
		self
	}

	/// Enable or disable drag & drop
	pub fn with_drag_drop(mut self, enabled: bool) -> Self {
		self.enable_drag_drop = enabled;
		self
	}

	/// Add custom CSS class
	pub fn with_css_class(mut self, css_class: impl Into<String>) -> Self {
		self.css_class = css_class.into();
		self
	}

	/// Get accept attribute for HTML input
	pub fn accept_attribute(&self) -> String {
		self.allowed_formats
			.iter()
			.map(|f| f.mime_type())
			.collect::<Vec<_>>()
			.join(",")
	}
}

/// Widget configuration for form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
	/// Widget type
	pub widget_type: WidgetType,
	/// HTML attributes
	pub attrs: HashMap<String, String>,
	/// Widget-specific options
	pub options: HashMap<String, serde_json::Value>,
}

impl Widget {
	/// Create a new widget
	pub fn new(widget_type: WidgetType) -> Self {
		Self {
			widget_type,
			attrs: HashMap::new(),
			options: HashMap::new(),
		}
	}

	/// Add HTML attribute
	pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.attrs.insert(key.into(), value.into());
		self
	}

	/// Add widget option
	pub fn with_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.options.insert(key.into(), value);
		self
	}

	/// Render widget to HTML
	pub fn render(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		match &self.widget_type {
			WidgetType::TextInput => self.render_text_input(name, value),
			WidgetType::TextArea { rows, cols } => self.render_textarea(name, value, *rows, *cols),
			WidgetType::Select { choices } => self.render_select(name, value, choices),
			WidgetType::CheckboxInput => self.render_checkbox(name, value),
			WidgetType::RadioSelect { choices } => self.render_radio(name, value, choices),
			WidgetType::DateInput => self.render_date_input(name, value),
			WidgetType::TimeInput => self.render_time_input(name, value),
			WidgetType::DateTimeInput => self.render_datetime_input(name, value),
			WidgetType::FileInput => self.render_file_input(name),
			WidgetType::HiddenInput => self.render_hidden_input(name, value),
			WidgetType::EmailInput => self.render_email_input(name, value),
			WidgetType::NumberInput => self.render_number_input(name, value),
			WidgetType::ColorPicker => self.render_color_picker(name, value),
			WidgetType::RichTextEditor => self.render_rich_text_editor(name, value),
			WidgetType::RichTextEditorWidget { config } => {
				self.render_rich_text_editor_widget(name, value, config)
			}
			WidgetType::ImageUploadWidget { config } => {
				self.render_image_upload_widget(name, value, config)
			}
			WidgetType::MultiSelect { choices } => self.render_multi_select(name, value, choices),
		}
	}

	fn render_attrs(&self) -> String {
		self.attrs
			.iter()
			.map(|(k, v)| format!("{}=\"{}\"", k, v))
			.collect::<Vec<_>>()
			.join(" ")
	}

	fn render_text_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"text\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_textarea(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		rows: usize,
		cols: usize,
	) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();
		format!(
			"<textarea name=\"{}\" rows=\"{}\" cols=\"{}\" {}>{}</textarea>",
			name, rows, cols, attrs, value_str
		)
	}

	fn render_select(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		choices: &[(String, String)],
	) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();
		let options = choices
			.iter()
			.map(|(val, label)| {
				let selected = if val == value_str { " selected" } else { "" };
				format!("<option value=\"{}\"{}>{}</option>", val, selected, label)
			})
			.collect::<Vec<_>>()
			.join("");
		format!("<select name=\"{}\" {}>{}</select>", name, attrs, options)
	}

	fn render_checkbox(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let checked = value.and_then(|v| v.as_bool()).unwrap_or(false);
		let checked_attr = if checked { " checked" } else { "" };
		let attrs = self.render_attrs();
		format!(
			"<input type=\"checkbox\" name=\"{}\" value=\"true\" {}{} />",
			name, attrs, checked_attr
		)
	}

	fn render_radio(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		choices: &[(String, String)],
	) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();
		choices
			.iter()
			.map(|(val, label)| {
				let checked = if val == value_str { " checked" } else { "" };
				format!(
					"<label><input type=\"radio\" name=\"{}\" value=\"{}\" {}{} /> {}</label>",
					name, val, attrs, checked, label
				)
			})
			.collect::<Vec<_>>()
			.join("<br>")
	}

	fn render_date_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"date\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_time_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"time\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_datetime_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"datetime-local\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_file_input(&self, name: &str) -> String {
		let attrs = self.render_attrs();
		format!("<input type=\"file\" name=\"{}\" {} />", name, attrs)
	}

	fn render_hidden_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		format!(
			"<input type=\"hidden\" name=\"{}\" value=\"{}\" />",
			name, value_str
		)
	}

	fn render_email_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.replace('"', "&quot;");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"email\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_number_input(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value.map(|v| v.to_string()).unwrap_or_default();
		let attrs = self.render_attrs();
		format!(
			"<input type=\"number\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_color_picker(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("#000000");
		let attrs = self.render_attrs();
		format!(
			"<input type=\"color\" name=\"{}\" value=\"{}\" {} />",
			name, value_str, attrs
		)
	}

	fn render_rich_text_editor(&self, name: &str, value: Option<&serde_json::Value>) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();
		format!(
			"<textarea name=\"{}\" class=\"rich-text-editor\" {}>{}</textarea>",
			name, attrs, value_str
		)
	}

	fn render_rich_text_editor_widget(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		config: &RichTextEditorConfig,
	) -> String {
		let value_str = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();

		let editor_data = serde_json::json!({
			"type": match config.editor_type {
				EditorType::TinyMCE => "tinymce",
				EditorType::CKEditor => "ckeditor",
				EditorType::Quill => "quill",
				EditorType::Simple => "simple",
			},
			"toolbar": config.toolbar,
			"maxLength": config.max_length,
			"allowedTags": config.allowed_tags,
			"fileUpload": config.file_upload_enabled,
		});

		format!(
			"<textarea name=\"{}\" class=\"{}\" data-editor='{}' {}>{}</textarea>",
			name,
			config.css_class,
			editor_data.to_string().replace('\'', "&apos;"),
			attrs,
			value_str
		)
	}

	fn render_image_upload_widget(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		config: &ImageUploadConfig,
	) -> String {
		let current_url = value.and_then(|v| v.as_str()).unwrap_or("");
		let attrs = self.render_attrs();

		let widget_data = serde_json::json!({
			"previewWidth": config.preview_width,
			"previewHeight": config.preview_height,
			"maxFileSize": config.max_file_size,
			"enableCrop": config.enable_crop,
			"enableResize": config.enable_resize,
			"enableDragDrop": config.enable_drag_drop,
			"generateThumbnail": config.generate_thumbnail,
			"thumbnailWidth": config.thumbnail_width,
			"thumbnailHeight": config.thumbnail_height,
		});

		let accept = config.accept_attribute();

		let preview_html = if !current_url.is_empty() {
			format!(
				"<div class=\"image-preview\" style=\"max-width: {}px; max-height: {}px;\">\
                    <img src=\"{}\" alt=\"Preview\" style=\"max-width: 100%; max-height: 100%;\" />\
                 </div>",
				config.preview_width, config.preview_height, current_url
			)
		} else {
			format!(
				"<div class=\"image-preview\" style=\"max-width: {}px; max-height: {}px; border: 2px dashed #ccc; display: flex; align-items: center; justify-content: center;\">\
                    <span>No image selected</span>\
                 </div>",
				config.preview_width, config.preview_height
			)
		};

		format!(
			"<div class=\"{}\" data-config='{}'>\
                {}\
                <input type=\"file\" name=\"{}\" accept=\"{}\" {} />\
                <input type=\"hidden\" name=\"{}_current\" value=\"{}\" />\
             </div>",
			config.css_class,
			widget_data.to_string().replace('\'', "&apos;"),
			preview_html,
			name,
			accept,
			attrs,
			name,
			current_url
		)
	}

	fn render_multi_select(
		&self,
		name: &str,
		value: Option<&serde_json::Value>,
		choices: &[(String, String)],
	) -> String {
		let selected_values: Vec<String> = value
			.and_then(|v| v.as_array())
			.map(|arr| {
				arr.iter()
					.filter_map(|v| v.as_str().map(String::from))
					.collect()
			})
			.unwrap_or_default();

		let attrs = self.render_attrs();
		let options = choices
			.iter()
			.map(|(val, label)| {
				let selected = if selected_values.contains(val) {
					" selected"
				} else {
					""
				};
				format!("<option value=\"{}\"{}>{}</option>", val, selected, label)
			})
			.collect::<Vec<_>>()
			.join("");

		format!(
			"<select name=\"{}\" multiple {}>{}</select>",
			name, attrs, options
		)
	}
}

/// Types of widgets available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
	/// Standard text input
	TextInput,
	/// Multi-line text area
	TextArea { rows: usize, cols: usize },
	/// Dropdown select
	Select { choices: Vec<(String, String)> },
	/// Checkbox
	CheckboxInput,
	/// Radio buttons
	RadioSelect { choices: Vec<(String, String)> },
	/// Date picker
	DateInput,
	/// Time picker
	TimeInput,
	/// DateTime picker
	DateTimeInput,
	/// File upload
	FileInput,
	/// Hidden input
	HiddenInput,
	/// Email input with validation
	EmailInput,
	/// Number input
	NumberInput,
	/// Color picker
	ColorPicker,
	/// Rich text editor (WYSIWYG)
	RichTextEditor,
	/// Rich text editor with advanced configuration
	RichTextEditorWidget { config: RichTextEditorConfig },
	/// Image upload widget with preview
	ImageUploadWidget { config: ImageUploadConfig },
	/// Multiple select dropdown
	MultiSelect { choices: Vec<(String, String)> },
}

/// Widget factory for creating common widgets
pub struct WidgetFactory;

impl WidgetFactory {
	/// Create a text input widget
	pub fn text_input() -> Widget {
		Widget::new(WidgetType::TextInput).with_attr("class", "form-control")
	}

	/// Create a textarea widget
	pub fn textarea(rows: usize, cols: usize) -> Widget {
		Widget::new(WidgetType::TextArea { rows, cols }).with_attr("class", "form-control")
	}

	/// Create a select widget
	pub fn select(choices: Vec<(String, String)>) -> Widget {
		Widget::new(WidgetType::Select { choices }).with_attr("class", "form-select")
	}

	/// Create a checkbox widget
	pub fn checkbox() -> Widget {
		Widget::new(WidgetType::CheckboxInput).with_attr("class", "form-check-input")
	}

	/// Create a radio select widget
	pub fn radio_select(choices: Vec<(String, String)>) -> Widget {
		Widget::new(WidgetType::RadioSelect { choices }).with_attr("class", "form-check-input")
	}

	/// Create a date input widget
	pub fn date_input() -> Widget {
		Widget::new(WidgetType::DateInput).with_attr("class", "form-control")
	}

	/// Create an email input widget
	pub fn email_input() -> Widget {
		Widget::new(WidgetType::EmailInput).with_attr("class", "form-control")
	}

	/// Create a number input widget
	pub fn number_input() -> Widget {
		Widget::new(WidgetType::NumberInput).with_attr("class", "form-control")
	}

	/// Create a color picker widget
	pub fn color_picker() -> Widget {
		Widget::new(WidgetType::ColorPicker).with_attr("class", "form-control form-control-color")
	}

	/// Create a rich text editor widget
	pub fn rich_text_editor() -> Widget {
		Widget::new(WidgetType::RichTextEditor).with_attr("class", "form-control rich-text-editor")
	}

	/// Create a rich text editor widget with configuration
	pub fn rich_text_editor_widget(config: RichTextEditorConfig) -> Widget {
		Widget::new(WidgetType::RichTextEditorWidget { config }).with_attr("class", "form-control")
	}

	/// Create a TinyMCE rich text editor widget
	pub fn tinymce_editor() -> Widget {
		let config = RichTextEditorConfig::new(EditorType::TinyMCE);
		Self::rich_text_editor_widget(config)
	}

	/// Create a CKEditor rich text editor widget
	pub fn ckeditor() -> Widget {
		let config = RichTextEditorConfig::new(EditorType::CKEditor);
		Self::rich_text_editor_widget(config)
	}

	/// Create a Quill rich text editor widget
	pub fn quill_editor() -> Widget {
		let config = RichTextEditorConfig::new(EditorType::Quill);
		Self::rich_text_editor_widget(config)
	}

	/// Create an image upload widget with configuration
	pub fn image_upload_widget(config: ImageUploadConfig) -> Widget {
		Widget::new(WidgetType::ImageUploadWidget { config }).with_attr("class", "form-control")
	}

	/// Create an image upload widget with default configuration
	pub fn image_upload() -> Widget {
		Self::image_upload_widget(ImageUploadConfig::default())
	}

	/// Create a multi-select widget
	pub fn multi_select(choices: Vec<(String, String)>) -> Widget {
		Widget::new(WidgetType::MultiSelect { choices })
			.with_attr("class", "form-select")
			.with_attr("size", "5")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_widget_new() {
		let widget = Widget::new(WidgetType::TextInput);
		assert!(matches!(widget.widget_type, WidgetType::TextInput));
		assert!(widget.attrs.is_empty());
	}

	#[test]
	fn test_widget_with_attr() {
		let widget = Widget::new(WidgetType::TextInput)
			.with_attr("class", "form-control")
			.with_attr("placeholder", "Enter text");

		assert_eq!(widget.attrs.len(), 2);
		assert_eq!(widget.attrs.get("class"), Some(&String::from("form-control")));
	}

	#[test]
	fn test_render_text_input() {
		let widget = Widget::new(WidgetType::TextInput).with_attr("class", "form-control");

		let html = widget.render(
			"username",
			Some(&serde_json::Value::String(String::from("alice"))),
		);

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);
		assert!(
			html.ends_with("/>"),
			"HTML should end with self-closing tag />, got: {}",
			&html[html.len().saturating_sub(10)..]
		);

		let expected_parts = [
			"type=\"text\"",
			"name=\"username\"",
			"value=\"alice\"",
			"class=\"form-control\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_textarea() {
		let widget = Widget::new(WidgetType::TextArea { rows: 5, cols: 40 });
		let html = widget.render("bio", Some(&serde_json::Value::String(String::from("Hello"))));

		assert!(
			html.starts_with("<textarea"),
			"HTML should start with <textarea tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("</textarea>"),
			"HTML should end with </textarea>, got: {}",
			&html[html.len().saturating_sub(20)..]
		);

		let expected_parts = [
			"name=\"bio\"",
			"rows=\"5\"",
			"cols=\"40\"",
			">Hello</textarea>",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_select() {
		let choices = vec![
			(String::from("active"), String::from("Active")),
			(String::from("inactive"), String::from("Inactive")),
		];
		let widget = Widget::new(WidgetType::Select { choices });
		let html = widget.render(
			"status",
			Some(&serde_json::Value::String(String::from("active"))),
		);

		assert!(
			html.starts_with("<select"),
			"HTML should start with <select tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("</select>"),
			"HTML should end with </select>, got: {}",
			&html[html.len().saturating_sub(20)..]
		);

		let expected_parts = [
			"name=\"status\"",
			"value=\"active\" selected",
			">Active</option>",
			"value=\"inactive\"",
			">Inactive</option>",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_checkbox() {
		let widget = Widget::new(WidgetType::CheckboxInput);
		let html = widget.render("is_active", Some(&serde_json::Value::Bool(true)));

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);

		let expected_parts = [
			"type=\"checkbox\"",
			"name=\"is_active\"",
			"value=\"true\"",
			"checked",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_date_input() {
		let widget = Widget::new(WidgetType::DateInput);
		let html = widget.render(
			"birth_date",
			Some(&serde_json::Value::String(String::from("2025-01-01"))),
		);

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("/>"),
			"HTML should end with self-closing tag />, got: {}",
			&html[html.len().saturating_sub(10)..]
		);

		let expected_parts = [
			"type=\"date\"",
			"name=\"birth_date\"",
			"value=\"2025-01-01\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_email_input() {
		let widget = Widget::new(WidgetType::EmailInput);
		let html = widget.render(
			"email",
			Some(&serde_json::Value::String(String::from("test@example.com"))),
		);

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("/>"),
			"HTML should end with self-closing tag />, got: {}",
			&html[html.len().saturating_sub(10)..]
		);

		let expected_parts = [
			"type=\"email\"",
			"name=\"email\"",
			"value=\"test@example.com\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_hidden_input() {
		let widget = Widget::new(WidgetType::HiddenInput);
		let html = widget.render("id", Some(&serde_json::Value::String(String::from("123"))));

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("/>"),
			"HTML should end with self-closing tag />, got: {}",
			&html[html.len().saturating_sub(10)..]
		);

		let expected_parts = ["type=\"hidden\"", "name=\"id\"", "value=\"123\""];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_widget_factory_text_input() {
		let widget = WidgetFactory::text_input();
		assert!(matches!(widget.widget_type, WidgetType::TextInput));
		assert_eq!(widget.attrs.get("class"), Some(&String::from("form-control")));
	}

	#[test]
	fn test_widget_factory_select() {
		let choices = vec![
			(String::from("1"), String::from("Option 1")),
			(String::from("2"), String::from("Option 2")),
		];
		let widget = WidgetFactory::select(choices);

		if let WidgetType::Select { choices } = &widget.widget_type {
			assert_eq!(choices.len(), 2);
		} else {
			panic!("Expected Select widget type");
		}
	}

	#[test]
	fn test_render_multi_select() {
		let choices = vec![
			(String::from("tag1"), String::from("Tag 1")),
			(String::from("tag2"), String::from("Tag 2")),
			(String::from("tag3"), String::from("Tag 3")),
		];
		let widget = Widget::new(WidgetType::MultiSelect { choices });

		let selected = serde_json::json!(["tag1", "tag3"]);
		let html = widget.render("tags", Some(&selected));

		assert!(
			html.starts_with("<select"),
			"HTML should start with <select tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("</select>"),
			"HTML should end with </select>, got: {}",
			&html[html.len().saturating_sub(20)..]
		);

		let expected_parts = [
			"name=\"tags\"",
			"multiple",
			"value=\"tag1\" selected",
			"value=\"tag3\" selected",
			">Tag 1</option>",
			">Tag 2</option>",
			">Tag 3</option>",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}

		assert!(
			!html.contains("value=\"tag2\" selected"),
			"Tag 2 should not be selected, got: {}",
			html
		);
	}

	#[test]
	fn test_render_color_picker() {
		let widget = Widget::new(WidgetType::ColorPicker);
		let html = widget.render(
			"color",
			Some(&serde_json::Value::String(String::from("#ff0000"))),
		);

		assert!(
			html.starts_with("<input"),
			"HTML should start with <input tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("/>"),
			"HTML should end with self-closing tag />, got: {}",
			&html[html.len().saturating_sub(10)..]
		);

		let expected_parts = ["type=\"color\"", "name=\"color\"", "value=\"#ff0000\""];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_rich_text_editor_config_default() {
		let config = RichTextEditorConfig::default();
		assert_eq!(config.editor_type, EditorType::TinyMCE);
		assert_eq!(
			config.toolbar,
			"bold italic underline | link image | bullist numlist"
		);
		assert!(!config.file_upload_enabled);
	}

	#[test]
	fn test_rich_text_editor_config_builder() {
		let config = RichTextEditorConfig::new(EditorType::Quill)
			.with_toolbar("bold italic | link")
			.with_max_length(5000)
			.with_allowed_tags("p,strong,em")
			.with_file_upload(true);

		assert_eq!(config.editor_type, EditorType::Quill);
		assert_eq!(config.toolbar, "bold italic | link");
		assert_eq!(config.max_length, Some(5000));
		assert_eq!(config.allowed_tags, Some(String::from("p,strong,em")));
		assert!(config.file_upload_enabled);
	}

	#[test]
	fn test_render_rich_text_editor_widget() {
		let config = RichTextEditorConfig::new(EditorType::CKEditor);
		let widget = Widget::new(WidgetType::RichTextEditorWidget { config });
		let html = widget.render(
			"content",
			Some(&serde_json::Value::String(String::from("Hello"))),
		);

		assert!(
			html.starts_with("<textarea"),
			"HTML should start with <textarea tag, got: {}",
			&html[..50.min(html.len())]
		);

		assert!(
			html.ends_with("</textarea>"),
			"HTML should end with </textarea>, got: {}",
			&html[html.len().saturating_sub(20)..]
		);

		let expected_parts = [
			"name=\"content\"",
			">Hello</textarea>",
			"data-editor",
			"class=\"rich-text-editor\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_image_format_mime_type() {
		assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
		assert_eq!(ImageFormat::Png.mime_type(), "image/png");
		assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");
		assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
	}

	#[test]
	fn test_image_format_extension() {
		assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
		assert_eq!(ImageFormat::Png.extension(), "png");
		assert_eq!(ImageFormat::Gif.extension(), "gif");
		assert_eq!(ImageFormat::WebP.extension(), "webp");
	}

	#[test]
	fn test_image_upload_config_default() {
		let config = ImageUploadConfig::default();
		assert_eq!(config.preview_width, 300);
		assert_eq!(config.preview_height, 300);
		assert_eq!(config.max_file_size, 5 * 1024 * 1024);
		assert!(config.generate_thumbnail);
		assert!(config.enable_crop);
		assert!(config.enable_resize);
		assert!(config.enable_drag_drop);
	}

	#[test]
	fn test_image_upload_config_builder() {
		let config = ImageUploadConfig::new()
			.with_preview_size(400, 400)
			.with_formats(vec![ImageFormat::Jpeg, ImageFormat::Png])
			.with_max_size(10 * 1024 * 1024)
			.with_thumbnail(false)
			.with_crop(false)
			.with_resize(false)
			.with_drag_drop(false);

		assert_eq!(config.preview_width, 400);
		assert_eq!(config.preview_height, 400);
		assert_eq!(config.allowed_formats.len(), 2);
		assert_eq!(config.max_file_size, 10 * 1024 * 1024);
		assert!(!config.generate_thumbnail);
		assert!(!config.enable_crop);
		assert!(!config.enable_resize);
		assert!(!config.enable_drag_drop);
	}

	#[test]
	fn test_image_upload_config_accept_attribute() {
		let config =
			ImageUploadConfig::new().with_formats(vec![ImageFormat::Jpeg, ImageFormat::Png]);

		let accept = config.accept_attribute();

		assert!(
			accept.contains("image/jpeg"),
			"Accept attribute should contain 'image/jpeg', got: {}",
			accept
		);
		assert!(
			accept.contains("image/png"),
			"Accept attribute should contain 'image/png', got: {}",
			accept
		);
		assert_eq!(
			accept.matches(',').count(),
			1,
			"Accept attribute should have exactly one comma separator, got: {}",
			accept
		);
	}

	#[test]
	fn test_render_image_upload_widget() {
		let config = ImageUploadConfig::default();
		let widget = Widget::new(WidgetType::ImageUploadWidget { config });
		let html = widget.render(
			"photo",
			Some(&serde_json::Value::String(String::from("/uploads/photo.jpg"))),
		);

		assert!(
			html.starts_with("<div"),
			"HTML should start with <div tag, got: {}",
			&html[..50.min(html.len())]
		);

		let expected_parts = [
			"<div class=\"image-upload-widget\"",
			"data-config",
			"input type=\"file\"",
			"name=\"photo\"",
			"accept=",
			"image-preview",
			"/uploads/photo.jpg",
			"input type=\"hidden\"",
			"name=\"photo_current\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_render_image_upload_widget_no_value() {
		let config = ImageUploadConfig::default();
		let widget = Widget::new(WidgetType::ImageUploadWidget { config });
		let html = widget.render("photo", None);

		assert!(
			html.starts_with("<div"),
			"HTML should start with <div tag, got: {}",
			&html[..50.min(html.len())]
		);

		let expected_parts = [
			"<div class=\"image-upload-widget\"",
			"input type=\"file\"",
			"name=\"photo\"",
			"No image selected",
			"border: 2px dashed #ccc",
			"input type=\"hidden\"",
		];
		for part in &expected_parts {
			assert!(
				html.contains(part),
				"HTML should contain '{}', got: {}",
				part,
				html
			);
		}
	}

	#[test]
	fn test_widget_factory_tinymce() {
		let widget = WidgetFactory::tinymce_editor();
		if let WidgetType::RichTextEditorWidget { config } = &widget.widget_type {
			assert_eq!(
				config.editor_type,
				EditorType::TinyMCE,
				"Editor type should be TinyMCE"
			);
		} else {
			panic!(
				"Expected RichTextEditorWidget but got: {:?}",
				widget.widget_type
			);
		}
	}

	#[test]
	fn test_widget_factory_ckeditor() {
		let widget = WidgetFactory::ckeditor();
		if let WidgetType::RichTextEditorWidget { config } = &widget.widget_type {
			assert_eq!(
				config.editor_type,
				EditorType::CKEditor,
				"Editor type should be CKEditor"
			);
		} else {
			panic!(
				"Expected RichTextEditorWidget but got: {:?}",
				widget.widget_type
			);
		}
	}

	#[test]
	fn test_widget_factory_quill() {
		let widget = WidgetFactory::quill_editor();
		if let WidgetType::RichTextEditorWidget { config } = &widget.widget_type {
			assert_eq!(
				config.editor_type,
				EditorType::Quill,
				"Editor type should be Quill"
			);
		} else {
			panic!(
				"Expected RichTextEditorWidget but got: {:?}",
				widget.widget_type
			);
		}
	}

	#[test]
	fn test_widget_factory_image_upload() {
		let widget = WidgetFactory::image_upload();
		assert!(
			matches!(widget.widget_type, WidgetType::ImageUploadWidget { .. }),
			"Expected ImageUploadWidget but got: {:?}",
			widget.widget_type
		);
	}

	#[test]
	fn test_image_upload_max_file_size_validation() {
		let config = ImageUploadConfig::new().with_max_size(1024 * 1024); // 1MB

		assert_eq!(
			config.max_file_size,
			1024 * 1024,
			"Max file size should be 1MB (1048576 bytes), got: {}",
			config.max_file_size
		);
	}

	#[test]
	fn test_rich_text_editor_different_types() {
		let types = vec![
			EditorType::TinyMCE,
			EditorType::CKEditor,
			EditorType::Quill,
			EditorType::Simple,
		];

		for editor_type in types {
			let config = RichTextEditorConfig::new(editor_type.clone());
			assert_eq!(
				config.editor_type, editor_type,
				"Editor type should match the configured type: {:?}",
				editor_type
			);
		}
	}
}
