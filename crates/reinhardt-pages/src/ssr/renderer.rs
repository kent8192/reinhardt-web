//! SSR Renderer for Component-based server-side rendering.

use super::markers::HydrationMarker;
use super::state::SsrState;
use crate::auth::AuthData;
use crate::component::{Component, IntoView, View};

/// Options for SSR rendering.
#[derive(Debug, Clone)]
pub struct SsrOptions {
	/// Whether to include hydration markers.
	pub include_hydration_markers: bool,
	/// Whether to minify the output.
	pub minify: bool,
	/// Whether to include SSR state script.
	pub include_state_script: bool,
	/// Custom document title.
	pub title: Option<String>,
	/// Custom meta tags.
	pub meta_tags: Vec<(String, String)>,
	/// Custom CSS links.
	pub css_links: Vec<String>,
	/// Custom JS scripts.
	pub js_scripts: Vec<String>,
	/// Language attribute for HTML element.
	pub lang: String,
	/// CSRF token to embed.
	pub csrf_token: Option<String>,
	/// Authentication data to embed.
	pub auth_data: Option<AuthData>,
}

impl Default for SsrOptions {
	fn default() -> Self {
		Self {
			include_hydration_markers: true,
			minify: false,
			include_state_script: true,
			title: None,
			meta_tags: Vec::new(),
			css_links: Vec::new(),
			js_scripts: Vec::new(),
			lang: "en".to_string(),
			csrf_token: None,
			auth_data: None,
		}
	}
}

impl SsrOptions {
	/// Creates new default options.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the document title.
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}

	/// Adds a meta tag.
	pub fn meta(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
		self.meta_tags.push((name.into(), content.into()));
		self
	}

	/// Adds a CSS link.
	pub fn css(mut self, href: impl Into<String>) -> Self {
		self.css_links.push(href.into());
		self
	}

	/// Adds a JS script.
	pub fn js(mut self, src: impl Into<String>) -> Self {
		self.js_scripts.push(src.into());
		self
	}

	/// Sets the language.
	pub fn lang(mut self, lang: impl Into<String>) -> Self {
		self.lang = lang.into();
		self
	}

	/// Disables hydration markers.
	pub fn no_hydration(mut self) -> Self {
		self.include_hydration_markers = false;
		self
	}

	/// Enables minification.
	pub fn minify(mut self) -> Self {
		self.minify = true;
		self
	}

	/// Sets the CSRF token.
	pub fn csrf(mut self, token: impl Into<String>) -> Self {
		self.csrf_token = Some(token.into());
		self
	}

	/// Sets the authentication data.
	pub fn auth(mut self, auth_data: AuthData) -> Self {
		self.auth_data = Some(auth_data);
		self
	}
}

/// The main SSR renderer.
pub struct SsrRenderer {
	options: SsrOptions,
	state: SsrState,
}

impl Default for SsrRenderer {
	fn default() -> Self {
		Self::new()
	}
}

impl SsrRenderer {
	/// Creates a new renderer with default options.
	pub fn new() -> Self {
		Self {
			options: SsrOptions::default(),
			state: SsrState::new(),
		}
	}

	/// Creates a renderer with custom options.
	pub fn with_options(options: SsrOptions) -> Self {
		Self {
			options,
			state: SsrState::new(),
		}
	}

	/// Returns a reference to the SSR state.
	pub fn state(&self) -> &SsrState {
		&self.state
	}

	/// Returns a mutable reference to the SSR state.
	pub fn state_mut(&mut self) -> &mut SsrState {
		&mut self.state
	}

	/// Renders a component to an HTML string.
	pub fn render<C: Component>(&mut self, component: &C) -> String {
		let view = component.render();
		self.render_view(&view)
	}

	/// Renders an IntoView to an HTML string.
	pub fn render_into_view<V: IntoView>(&mut self, view: V) -> String {
		let view = view.into_view();
		self.render_view(&view)
	}

	/// Renders a View to an HTML string.
	pub fn render_view(&self, view: &View) -> String {
		view.render_to_string()
	}

	/// Renders a component to a full HTML page.
	pub fn render_page<C: Component>(&mut self, component: &C) -> String {
		let content = self.render(component);
		self.wrap_in_html(&content)
	}

	/// Renders an IntoView to a full HTML page.
	pub fn render_page_into_view<V: IntoView>(&mut self, view: V) -> String {
		let content = self.render_into_view(view);
		self.wrap_in_html(&content)
	}

	/// Wraps content in a full HTML document.
	pub fn wrap_in_html(&self, content: &str) -> String {
		let mut html = String::with_capacity(content.len() + 1024);

		// DOCTYPE and html opening
		html.push_str("<!DOCTYPE html>\n");
		html.push_str(&format!("<html lang=\"{}\">\n", self.options.lang));

		// Head section
		html.push_str("<head>\n");
		html.push_str("<meta charset=\"UTF-8\">\n");
		html.push_str(
			"<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
		);

		// Title
		if let Some(ref title) = self.options.title {
			html.push_str(&format!("<title>{}</title>\n", html_escape(title)));
		}

		// Custom meta tags
		for (name, content) in &self.options.meta_tags {
			html.push_str(&format!(
				"<meta name=\"{}\" content=\"{}\">\n",
				html_escape(name),
				html_escape(content)
			));
		}

		// CSRF token meta tag
		if let Some(ref token) = self.options.csrf_token {
			html.push_str(&format!(
				"<meta name=\"csrf-token\" content=\"{}\">\n",
				html_escape(token)
			));
		}

		// CSS links
		for href in &self.options.css_links {
			html.push_str(&format!(
				"<link rel=\"stylesheet\" href=\"{}\">\n",
				html_escape(href)
			));
		}

		html.push_str("</head>\n");

		// Body section
		html.push_str("<body>\n");

		// Main content
		html.push_str("<div id=\"app\">");
		html.push_str(content);
		html.push_str("</div>\n");

		// Auth data script (if provided)
		if let Some(ref auth_data) = self.options.auth_data
			&& let Ok(json) = serde_json::to_string(auth_data)
		{
			html.push_str(&format!(
				"<script id=\"auth-data\" type=\"application/json\">{}</script>\n",
				json
			));
		}

		// SSR state script (if enabled)
		if self.options.include_state_script && !self.state.is_empty() {
			html.push_str(&self.state.to_script_tag());
			html.push('\n');
		}

		// JS scripts
		for src in &self.options.js_scripts {
			html.push_str(&format!("<script src=\"{}\"></script>\n", html_escape(src)));
		}

		html.push_str("</body>\n");
		html.push_str("</html>");

		if self.options.minify {
			minify_html(&html)
		} else {
			html
		}
	}

	/// Renders a component with hydration marker.
	pub fn render_with_marker<C: Component>(&mut self, component: &C) -> String {
		let marker = HydrationMarker::with_component(C::name());
		let view = component.render();
		let content = view.render_to_string();

		if self.options.include_hydration_markers {
			format!("<div {}>{}</div>", marker.to_attr_string(), content)
		} else {
			content
		}
	}
}

/// Simple HTML escape function.
fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
}

/// Simple HTML minification (removes extra whitespace).
fn minify_html(html: &str) -> String {
	// Simple minification: collapse multiple whitespace and remove newlines
	let mut result = String::with_capacity(html.len());
	let mut prev_was_whitespace = false;
	let mut in_pre = false;

	for c in html.chars() {
		// Track <pre> tags to preserve whitespace inside them
		if html.contains("<pre") {
			in_pre = true;
		}
		if html.contains("</pre>") {
			in_pre = false;
		}

		if in_pre {
			result.push(c);
		} else if c.is_whitespace() {
			if !prev_was_whitespace {
				result.push(' ');
				prev_was_whitespace = true;
			}
		} else {
			result.push(c);
			prev_was_whitespace = false;
		}
	}

	result
}

/// Helper function for simple component rendering.
#[allow(dead_code)]
pub(super) fn render<C: Component>(component: &C) -> String {
	let mut renderer = SsrRenderer::new();
	renderer.render(component)
}

/// Helper function for rendering to a full HTML page.
#[allow(dead_code)]
pub(super) fn render_page<C: Component>(component: &C, options: SsrOptions) -> String {
	let mut renderer = SsrRenderer::with_options(options);
	renderer.render_page(component)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::component::ElementView;

	struct TestComponent {
		message: String,
	}

	impl Component for TestComponent {
		fn render(&self) -> View {
			ElementView::new("div")
				.attr("class", "test")
				.child(self.message.clone())
				.into_view()
		}

		fn name() -> &'static str {
			"TestComponent"
		}
	}

	#[test]
	fn test_ssr_options_default() {
		let opts = SsrOptions::default();
		assert!(opts.include_hydration_markers);
		assert!(!opts.minify);
		assert_eq!(opts.lang, "en");
	}

	#[test]
	fn test_ssr_options_builder() {
		let opts = SsrOptions::new()
			.title("Test Page")
			.lang("ja")
			.css("/styles.css")
			.js("/app.js")
			.meta("description", "A test page");

		assert_eq!(opts.title, Some("Test Page".to_string()));
		assert_eq!(opts.lang, "ja");
		assert_eq!(opts.css_links, vec!["/styles.css"]);
		assert_eq!(opts.js_scripts, vec!["/app.js"]);
		assert_eq!(
			opts.meta_tags,
			vec![("description".to_string(), "A test page".to_string())]
		);
	}

	#[test]
	fn test_ssr_renderer_render() {
		let component = TestComponent {
			message: "Hello".to_string(),
		};
		let mut renderer = SsrRenderer::new();
		let html = renderer.render(&component);
		assert_eq!(html, "<div class=\"test\">Hello</div>");
	}

	#[test]
	fn test_ssr_renderer_render_page() {
		let component = TestComponent {
			message: "World".to_string(),
		};
		let opts = SsrOptions::new().title("Test");
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page(&component);

		assert!(html.starts_with("<!DOCTYPE html>"));
		assert!(html.contains("<title>Test</title>"));
		assert!(html.contains("<div id=\"app\">"));
		assert!(html.contains("<div class=\"test\">World</div>"));
		assert!(html.ends_with("</html>"));
	}

	#[test]
	fn test_ssr_renderer_with_csrf() {
		let component = TestComponent {
			message: "Secure".to_string(),
		};
		let opts = SsrOptions::new().csrf("test-token-123");
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page(&component);

		assert!(html.contains("csrf-token"));
		assert!(html.contains("test-token-123"));
	}

	#[test]
	fn test_ssr_renderer_with_auth() {
		let component = TestComponent {
			message: "Auth".to_string(),
		};
		let auth = AuthData::authenticated(1, "testuser");
		let opts = SsrOptions::new().auth(auth);
		let mut renderer = SsrRenderer::with_options(opts);
		let html = renderer.render_page(&component);

		assert!(html.contains("auth-data"));
		assert!(html.contains("testuser"));
	}

	#[test]
	fn test_ssr_renderer_with_marker() {
		let component = TestComponent {
			message: "Hydrate".to_string(),
		};
		let mut renderer = SsrRenderer::new();
		let html = renderer.render_with_marker(&component);

		assert!(html.contains("data-rh-id"));
		assert!(html.contains("data-rh-component=\"TestComponent\""));
	}

	#[test]
	fn test_render_helper() {
		let component = TestComponent {
			message: "Helper".to_string(),
		};
		let html = render(&component);
		assert_eq!(html, "<div class=\"test\">Helper</div>");
	}

	#[test]
	fn test_html_escape() {
		assert_eq!(html_escape("<script>"), "&lt;script&gt;");
		assert_eq!(html_escape("a&b"), "a&amp;b");
		assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
	}

	#[cfg(all(test, feature = "static"))]
	mod static_integration_tests {
		use super::*;
		#[cfg(feature = "static")]
		use reinhardt_static::template_integration::TemplateStaticConfig;
		use std::collections::HashMap;

		#[test]
		fn test_ssr_options_with_static_config() {
			let static_config = TemplateStaticConfig::new("/static/".to_string());

			let opts = SsrOptions::new()
				.title("Test")
				.css(static_config.resolve_url("css/style.css"))
				.js(static_config.resolve_url("js/app.js"));

			assert_eq!(opts.css_links, vec!["/static/css/style.css"]);
			assert_eq!(opts.js_scripts, vec!["/static/js/app.js"]);
		}

		#[test]
		fn test_ssr_options_with_manifest() {
			let mut manifest = HashMap::new();
			manifest.insert(
				"css/style.css".to_string(),
				"css/style.abc123.css".to_string(),
			);
			manifest.insert("js/app.js".to_string(), "js/app.def456.js".to_string());

			let static_config =
				TemplateStaticConfig::new("/static/".to_string()).with_manifest(manifest);

			let opts = SsrOptions::new()
				.css(static_config.resolve_url("css/style.css"))
				.js(static_config.resolve_url("js/app.js"));

			assert_eq!(opts.css_links, vec!["/static/css/style.abc123.css"]);
			assert_eq!(opts.js_scripts, vec!["/static/js/app.def456.js"]);
		}

		#[test]
		fn test_ssr_renderer_with_static_urls() {
			let static_config = TemplateStaticConfig::new("/static/".to_string());

			let opts = SsrOptions::new()
				.title("Static Test")
				.css(static_config.resolve_url("css/style.css"))
				.js(static_config.resolve_url("js/app.js"));

			let renderer = SsrRenderer::with_options(opts);
			let html = renderer.wrap_in_html("<div>Test</div>");

			assert!(html.contains("<link rel=\"stylesheet\" href=\"/static/css/style.css\">"));
			assert!(html.contains("<script src=\"/static/js/app.js\"></script>"));
		}
	}
}
