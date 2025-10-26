//! Syntax highlighting for API responses

use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use super::ColorScheme;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Syntax highlighter for code snippets
///
/// # Examples
///
/// ```
/// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
///
/// let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
/// let json = r#"{"name": "John", "age": 30}"#;
/// let highlighted = highlighter.highlight_json(json).unwrap();
/// assert!(highlighted.contains("name"));
/// ```
#[derive(Debug, Clone)]
pub struct SyntaxHighlighter {
    color_scheme: ColorScheme,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with the specified color scheme
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
    ///
    /// let highlighter = SyntaxHighlighter::new(ColorScheme::Monokai);
    /// ```
    pub fn new(color_scheme: ColorScheme) -> Self {
        Self { color_scheme }
    }

    /// Highlight JSON content
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
    ///
    /// let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
    /// let json = r#"{"key": "value"}"#;
    /// let result = highlighter.highlight_json(json);
    /// assert!(result.is_ok());
    /// ```
    pub fn highlight_json(&self, content: &str) -> Result<String, String> {
        self.highlight(content, "json")
    }

    /// Highlight code with the specified syntax
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::highlighter::{SyntaxHighlighter};
    /// use reinhardt_views_core::browsable_api::ColorScheme;
    ///
    /// let highlighter = SyntaxHighlighter::new(ColorScheme::Light);
    /// let code = "fn main() { println!(\"Hello\"); }";
    /// let result = highlighter.highlight(code, "rs");
    /// assert!(result.is_ok());
    /// ```
    pub fn highlight(&self, content: &str, syntax: &str) -> Result<String, String> {
        let syntax = SYNTAX_SET
            .find_syntax_by_extension(syntax)
            .ok_or_else(|| format!("Syntax not found: {}", syntax))?;

        let theme = &THEME_SET.themes[self.color_scheme.theme_name()];

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut html = String::new();

        for line in LinesWithEndings::from(content) {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &SYNTAX_SET)
                .map_err(|e| e.to_string())?;
            let line_html = styled_line_to_highlighted_html(&ranges[..], IncludeBackground::No)
                .map_err(|e| e.to_string())?;
            html.push_str(&line_html);
        }

        Ok(html)
    }

    /// Wrap highlighted content in a styled pre element
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
    ///
    /// let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
    /// let json = r#"{"test": true}"#;
    /// let wrapped = highlighter.highlight_and_wrap_json(json).unwrap();
    /// assert!(wrapped.contains("<pre"));
    /// assert!(wrapped.contains("</pre>"));
    /// ```
    pub fn highlight_and_wrap_json(&self, content: &str) -> Result<String, String> {
        let highlighted = self.highlight_json(content)?;
        Ok(self.wrap_in_pre(&highlighted))
    }

    /// Wrap content in a styled pre element
    fn wrap_in_pre(&self, content: &str) -> String {
        let bg_color = match self.color_scheme {
            ColorScheme::Dark | ColorScheme::Monokai | ColorScheme::SolarizedDark => "#282c34",
            ColorScheme::Light => "#f8f8f8",
            ColorScheme::SolarizedLight => "#fdf6e3",
        };

        format!(
            r#"<pre style="background-color: {}; color: #abb2bf; padding: 15px; border-radius: 4px; overflow-x: auto; font-family: 'Courier New', monospace; line-height: 1.5;">{}</pre>"#,
            bg_color, content
        )
    }

    /// Set a new color scheme
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
    ///
    /// let mut highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
    /// highlighter.set_color_scheme(ColorScheme::Light);
    /// ```
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.color_scheme = scheme;
    }

    /// Get the current color scheme
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views_core::browsable_api::{SyntaxHighlighter, ColorScheme};
    ///
    /// let highlighter = SyntaxHighlighter::new(ColorScheme::Monokai);
    /// assert_eq!(highlighter.color_scheme(), ColorScheme::Monokai);
    /// ```
    pub fn color_scheme(&self) -> ColorScheme {
        self.color_scheme
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new(ColorScheme::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
        assert_eq!(highlighter.color_scheme(), ColorScheme::Dark);
    }

    #[test]
    fn test_highlight_json() {
        let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
        let json = r#"{"name": "Alice", "age": 30}"#;
        let result = highlighter.highlight_json(json);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_highlight_with_syntax() {
        let highlighter = SyntaxHighlighter::new(ColorScheme::Light);
        let rust_code = "fn main() { println!(\"Hello, world!\"); }";
        let result = highlighter.highlight(rust_code, "rs");
        assert!(result.is_ok());
    }

    #[test]
    fn test_highlight_invalid_syntax() {
        let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
        let result = highlighter.highlight("code", "invalid_syntax");
        assert!(result.is_err());
    }

    #[test]
    fn test_highlight_and_wrap_json() {
        let highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
        let json = r#"{"test": true}"#;
        let result = highlighter.highlight_and_wrap_json(json);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("<pre"));
        assert!(html.contains("</pre>"));
    }

    #[test]
    fn test_set_color_scheme() {
        let mut highlighter = SyntaxHighlighter::new(ColorScheme::Dark);
        highlighter.set_color_scheme(ColorScheme::Monokai);
        assert_eq!(highlighter.color_scheme(), ColorScheme::Monokai);
    }

    #[test]
    fn test_default_highlighter() {
        let highlighter = SyntaxHighlighter::default();
        assert_eq!(highlighter.color_scheme(), ColorScheme::Dark);
    }
}
