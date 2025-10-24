//! Development error pages with enhanced debugging information
//!
//! Provides detailed error pages during development with stack traces,
//! source code context, and helpful debugging information.

use std::error::Error;
use std::fmt;

/// Development error handler
///
/// Generates detailed error pages with stack traces and debugging information.
/// Should only be enabled in development environments.
pub struct DevelopmentErrorHandler {
    /// Show full stack traces
    show_stack_trace: bool,
    /// Show source code context
    show_source: bool,
    /// Maximum lines of source code to show
    source_context_lines: usize,
}

impl DevelopmentErrorHandler {
    /// Create a new development error handler with default settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::DevelopmentErrorHandler;
    ///
    /// let handler = DevelopmentErrorHandler::new();
    /// ```
    pub fn new() -> Self {
        Self {
            show_stack_trace: true,
            show_source: true,
            source_context_lines: 5,
        }
    }

    /// Enable or disable stack trace display
    pub fn with_stack_trace(mut self, enable: bool) -> Self {
        self.show_stack_trace = enable;
        self
    }

    /// Enable or disable source code display
    pub fn with_source_context(mut self, enable: bool) -> Self {
        self.show_source = enable;
        self
    }

    /// Set the number of source context lines to show
    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.source_context_lines = lines;
        self
    }

    /// Format an error into an HTML error page
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::DevelopmentErrorHandler;
    /// use std::io;
    ///
    /// let handler = DevelopmentErrorHandler::new();
    /// let error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    /// let html = handler.format_error(&error);
    /// assert!(html.contains("File not found"));
    /// ```
    pub fn format_error(&self, error: &dyn Error) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html>\n");
        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"utf-8\">\n");
        html.push_str("  <title>Development Error</title>\n");
        html.push_str("  <style>\n");
        html.push_str(&self.error_page_styles());
        html.push_str("  </style>\n");
        html.push_str("</head>\n");
        html.push_str("<body>\n");

        html.push_str("  <div class=\"error-container\">\n");
        html.push_str("    <h1>Development Error</h1>\n");

        // Main error message
        html.push_str("    <div class=\"error-message\">\n");
        html.push_str(&format!("      <p><strong>Error:</strong> {}</p>\n", error));
        html.push_str("    </div>\n");

        // Stack trace
        if self.show_stack_trace {
            html.push_str(&self.format_stack_trace(error));
        }

        // Error chain
        if let Some(source) = error.source() {
            html.push_str("    <div class=\"error-chain\">\n");
            html.push_str("      <h2>Caused by:</h2>\n");
            html.push_str("      <ul>\n");

            let mut current = Some(source);
            while let Some(err) = current {
                html.push_str(&format!("        <li>{}</li>\n", err));
                current = err.source();
            }

            html.push_str("      </ul>\n");
            html.push_str("    </div>\n");
        }

        html.push_str("  </div>\n");
        html.push_str("</body>\n");
        html.push_str("</html>\n");

        html
    }

    /// Format a simple error message (plain text)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::DevelopmentErrorHandler;
    /// use std::io;
    ///
    /// let handler = DevelopmentErrorHandler::new();
    /// let error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    /// let text = handler.format_error_text(&error);
    /// assert!(text.contains("Error: File not found"));
    /// ```
    pub fn format_error_text(&self, error: &dyn Error) -> String {
        let mut text = String::new();

        text.push_str("Development Error\n");
        text.push_str("================\n\n");
        text.push_str(&format!("Error: {}\n", error));

        if let Some(source) = error.source() {
            text.push_str("\nCaused by:\n");

            let mut current = Some(source);
            while let Some(err) = current {
                text.push_str(&format!("  - {}\n", err));
                current = err.source();
            }
        }

        text
    }

    fn format_stack_trace(&self, _error: &dyn Error) -> String {
        let mut html = String::new();

        html.push_str("    <div class=\"stack-trace\">\n");
        html.push_str("      <h2>Stack Trace</h2>\n");
        html.push_str("      <pre>\n");

        // Note: Rust doesn't provide built-in backtrace capture for arbitrary errors
        // In a real implementation, you would use the backtrace crate or
        // std::backtrace::Backtrace when it's stabilized
        html.push_str("Stack trace capture not available\n");
        html.push_str("Hint: Enable RUST_BACKTRACE=1 environment variable to capture backtraces\n");

        html.push_str("      </pre>\n");
        html.push_str("    </div>\n");

        html
    }

    fn error_page_styles(&self) -> String {
        r#"
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      margin: 0;
      padding: 20px;
      background: #f5f5f5;
      color: #333;
    }
    .error-container {
      max-width: 1200px;
      margin: 0 auto;
      background: white;
      border-radius: 8px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.1);
      padding: 40px;
    }
    h1 {
      color: #d32f2f;
      margin-top: 0;
      border-bottom: 2px solid #d32f2f;
      padding-bottom: 10px;
    }
    h2 {
      color: #555;
      margin-top: 30px;
      font-size: 1.2em;
    }
    .error-message {
      background: #ffebee;
      border-left: 4px solid #d32f2f;
      padding: 15px;
      margin: 20px 0;
    }
    .error-message p {
      margin: 0;
    }
    .error-chain {
      background: #fff3e0;
      border-left: 4px solid #ff9800;
      padding: 15px;
      margin: 20px 0;
    }
    .error-chain ul {
      margin: 10px 0 0 0;
      padding-left: 20px;
    }
    .error-chain li {
      margin: 5px 0;
    }
    .stack-trace {
      background: #f5f5f5;
      border: 1px solid #ddd;
      border-radius: 4px;
      padding: 15px;
      margin: 20px 0;
    }
    .stack-trace pre {
      margin: 10px 0 0 0;
      overflow-x: auto;
      font-family: "Courier New", monospace;
      font-size: 0.9em;
      line-height: 1.5;
    }
"#
        .to_string()
    }
}

impl Default for DevelopmentErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for development server operations
#[derive(Debug)]
pub struct DevServerError {
    message: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl DevServerError {
    /// Create a new development server error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Create an error with a source
    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for DevServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for DevServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_new_handler() {
        let handler = DevelopmentErrorHandler::new();
        assert!(handler.show_stack_trace);
        assert!(handler.show_source);
        assert_eq!(handler.source_context_lines, 5);
    }

    #[test]
    fn test_handler_builder() {
        let handler = DevelopmentErrorHandler::new()
            .with_stack_trace(false)
            .with_source_context(false)
            .with_context_lines(10);

        assert!(!handler.show_stack_trace);
        assert!(!handler.show_source);
        assert_eq!(handler.source_context_lines, 10);
    }

    #[test]
    fn test_format_error_html() {
        let handler = DevelopmentErrorHandler::new();
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        let html = handler.format_error(&error);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Development Error"));
        assert!(html.contains("File not found"));
    }

    #[test]
    fn test_format_error_text() {
        let handler = DevelopmentErrorHandler::new();
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        let text = handler.format_error_text(&error);

        assert!(text.contains("Development Error"));
        assert!(text.contains("Error: File not found"));
    }

    #[test]
    fn test_format_with_disabled_stack_trace() {
        let handler = DevelopmentErrorHandler::new().with_stack_trace(false);
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        let html = handler.format_error(&error);

        assert!(!html.contains("Stack Trace"));
    }

    #[test]
    fn test_dev_server_error_new() {
        let error = DevServerError::new("Test error");
        assert_eq!(error.to_string(), "Test error");
        assert!(error.source().is_none());
    }

    #[test]
    fn test_dev_server_error_with_source() {
        let source = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = DevServerError::with_source("Failed to load file", source);

        assert_eq!(error.to_string(), "Failed to load file");
        assert!(error.source().is_some());
    }

    #[test]
    fn test_error_page_styles() {
        let handler = DevelopmentErrorHandler::new();
        let styles = handler.error_page_styles();

        assert!(styles.contains("body"));
        assert!(styles.contains(".error-container"));
        assert!(styles.contains(".error-message"));
    }

    #[test]
    fn test_default() {
        let handler = DevelopmentErrorHandler::default();
        assert!(handler.show_stack_trace);
        assert!(handler.show_source);
    }
}
