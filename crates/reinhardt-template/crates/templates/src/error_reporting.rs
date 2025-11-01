//! Enhanced error reporting for templates
//!
//! Provides detailed error messages with context, line numbers,
//! and helpful suggestions for fixing template errors.

use std::fmt;

/// Template error context
#[derive(Debug, Clone)]
pub struct TemplateErrorContext {
    /// Template name or file path
    pub template_name: String,
    /// Line number where the error occurred
    pub line: Option<usize>,
    /// Column number where the error occurred
    pub column: Option<usize>,
    /// The line of code that caused the error
    pub source_line: Option<String>,
    /// Additional context lines before the error
    pub context_before: Vec<String>,
    /// Additional context lines after the error
    pub context_after: Vec<String>,
    /// Error message
    pub message: String,
    /// Suggestion for fixing the error
    pub suggestion: Option<String>,
}

impl TemplateErrorContext {
    /// Create a new error context
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateErrorContext;
    ///
    /// let context = TemplateErrorContext::new(
    ///     "user_list.html",
    ///     10,
    ///     Some(5),
    ///     "Undefined variable 'username'",
    /// );
    /// assert_eq!(context.line, Some(10));
    /// ```
    pub fn new(
        template_name: impl Into<String>,
        line: usize,
        column: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            template_name: template_name.into(),
            line: Some(line),
            column,
            source_line: None,
            context_before: Vec::new(),
            context_after: Vec::new(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add source line
    pub fn with_source_line(mut self, line: impl Into<String>) -> Self {
        self.source_line = Some(line.into());
        self
    }

    /// Add context lines before the error
    pub fn with_context_before(mut self, lines: Vec<String>) -> Self {
        self.context_before = lines;
        self
    }

    /// Add context lines after the error
    pub fn with_context_after(mut self, lines: Vec<String>) -> Self {
        self.context_after = lines;
        self
    }

    /// Add a suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Format the error for display
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateErrorContext;
    ///
    /// let context = TemplateErrorContext::new(
    ///     "template.html",
    ///     10,
    ///     Some(5),
    ///     "Undefined variable",
    /// )
    /// .with_source_line("{{ invalid_var }}")
    /// .with_suggestion("Did you mean 'valid_var'?");
    ///
    /// let formatted = context.format();
    /// assert!(formatted.contains("template.html"));
    /// assert!(formatted.contains("line 10"));
    /// ```
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Template name and location
        output.push_str(&format!("Error in template: {}\n", self.template_name));
        if let Some(line) = self.line {
            output.push_str(&format!("  at line {}", line));
            if let Some(col) = self.column {
                output.push_str(&format!(", column {}", col));
            }
            output.push('\n');
        }
        output.push('\n');

        // Context before
        if !self.context_before.is_empty() {
            for (i, line) in self.context_before.iter().enumerate() {
                let line_num = self
                    .line
                    .unwrap_or(0)
                    .saturating_sub(self.context_before.len() - i);
                output.push_str(&format!("  {:4} | {}\n", line_num, line));
            }
        }

        // Error line
        if let Some(ref source_line) = self.source_line {
            let line_num = self.line.unwrap_or(0);
            output.push_str(&format!("  {:4} | {}\n", line_num, source_line));

            // Add marker for column
            if let Some(col) = self.column {
                output.push_str(&format!("       | {}^\n", " ".repeat(col)));
            }
        }

        // Context after
        if !self.context_after.is_empty() {
            for (i, line) in self.context_after.iter().enumerate() {
                let line_num = self.line.unwrap_or(0) + i + 1;
                output.push_str(&format!("  {:4} | {}\n", line_num, line));
            }
        }

        // Error message
        output.push('\n');
        output.push_str(&format!("Error: {}\n", self.message));

        // Suggestion
        if let Some(ref suggestion) = self.suggestion {
            output.push('\n');
            output.push_str(&format!("Suggestion: {}\n", suggestion));
        }

        output
    }
}

impl fmt::Display for TemplateErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Template error type
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// Undefined variable error
    UndefinedVariable {
        template: String,
        variable: String,
        line: usize,
        suggestion: Option<String>,
    },
    /// Syntax error
    SyntaxError {
        template: String,
        message: String,
        line: usize,
        column: Option<usize>,
    },
    /// Filter not found
    FilterNotFound {
        template: String,
        filter: String,
        line: usize,
    },
    /// Include/extends error
    IncludeError {
        template: String,
        included_template: String,
        message: String,
    },
    /// Generic error
    Generic { message: String },
}

impl TemplateError {
    /// Convert to error context
    pub fn to_context(&self) -> TemplateErrorContext {
        match self {
            TemplateError::UndefinedVariable {
                template,
                variable,
                line,
                suggestion,
            } => {
                let mut ctx = TemplateErrorContext::new(
                    template,
                    *line,
                    None,
                    format!("Undefined variable '{}'", variable),
                );
                if let Some(sug) = suggestion {
                    ctx = ctx.with_suggestion(sug);
                }
                ctx
            }
            TemplateError::SyntaxError {
                template,
                message,
                line,
                column,
            } => TemplateErrorContext::new(template, *line, *column, message),
            TemplateError::FilterNotFound {
                template,
                filter,
                line,
            } => TemplateErrorContext::new(
                template,
                *line,
                None,
                format!("Filter '{}' not found", filter),
            )
            .with_suggestion("Check if the filter is registered or imported"),
            TemplateError::IncludeError {
                template,
                included_template,
                message,
            } => TemplateErrorContext::new(
                template,
                0,
                None,
                format!("Failed to include '{}': {}", included_template, message),
            ),
            TemplateError::Generic { message } => {
                TemplateErrorContext::new("unknown", 0, None, message)
            }
        }
    }

    /// Format the error with context
    pub fn format(&self) -> String {
        self.to_context().format()
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl std::error::Error for TemplateError {}

/// Helper to suggest similar variable names
///
/// # Examples
///
/// ```
/// use reinhardt_templates::suggest_similar;
///
/// let available = vec!["username", "user_id", "email"];
/// let suggestion = suggest_similar("usrname", &available);
/// assert_eq!(suggestion, Some("username".to_string()));
/// ```
pub fn suggest_similar(input: &str, available: &[&str]) -> Option<String> {
    let input_lower = input.to_lowercase();
    let mut best_match: Option<(&str, usize)> = None;

    for &candidate in available {
        let distance = levenshtein_distance(&input_lower, &candidate.to_lowercase());
        if distance <= 3 {
            // Allow up to 3 character differences
            if let Some((_, best_dist)) = best_match {
                if distance < best_dist {
                    best_match = Some((candidate, distance));
                }
            } else {
                best_match = Some((candidate, distance));
            }
        }
    }

    best_match.map(|(name, _)| name.to_string())
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(matrix[i][j + 1] + 1, matrix[i + 1][j] + 1),
                matrix[i][j] + cost,
            );
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_error_context_new() {
        let context = TemplateErrorContext::new("template.html", 10, Some(5), "Test error");
        assert_eq!(context.template_name, "template.html");
        assert_eq!(context.line, Some(10));
        assert_eq!(context.column, Some(5));
        assert_eq!(context.message, "Test error");
    }

    #[test]
    fn test_template_error_context_with_source() {
        let context = TemplateErrorContext::new("template.html", 10, None, "Error")
            .with_source_line("{{ variable }}");
        assert_eq!(context.source_line, Some("{{ variable }}".to_string()));
    }

    #[test]
    fn test_template_error_context_with_suggestion() {
        let context = TemplateErrorContext::new("template.html", 10, None, "Error")
            .with_suggestion("Try this instead");
        assert_eq!(context.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_template_error_context_format() {
        let context = TemplateErrorContext::new("template.html", 10, Some(5), "Test error")
            .with_source_line("{{ invalid }}")
            .with_suggestion("Check variable name");

        let formatted = context.format();
        assert!(formatted.contains("template.html"));
        assert!(formatted.contains("line 10"));
        assert!(formatted.contains("Test error"));
        assert!(formatted.contains("Check variable name"));
    }

    #[test]
    fn test_suggest_similar() {
        let available = vec!["username", "user_id", "email"];
        assert_eq!(
            suggest_similar("usrname", &available),
            Some("username".to_string())
        );
        assert_eq!(
            suggest_similar("userid", &available),
            Some("user_id".to_string())
        );
        assert_eq!(suggest_similar("totally_different", &available), None);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_template_error_undefined_variable() {
        let error = TemplateError::UndefinedVariable {
            template: "test.html".to_string(),
            variable: "invalid_var".to_string(),
            line: 10,
            suggestion: Some("Did you mean 'valid_var'?".to_string()),
        };

        let formatted = error.format();
        assert!(formatted.contains("Undefined variable 'invalid_var'"));
        assert!(formatted.contains("Did you mean 'valid_var'?"));
    }

    #[test]
    fn test_template_error_syntax_error() {
        let error = TemplateError::SyntaxError {
            template: "test.html".to_string(),
            message: "Unexpected token".to_string(),
            line: 5,
            column: Some(10),
        };

        let formatted = error.format();
        assert!(formatted.contains("Unexpected token"));
        assert!(formatted.contains("line 5"));
    }

    #[test]
    fn test_template_error_filter_not_found() {
        let error = TemplateError::FilterNotFound {
            template: "test.html".to_string(),
            filter: "custom_filter".to_string(),
            line: 8,
        };

        let formatted = error.format();
        assert!(formatted.contains("Filter 'custom_filter' not found"));
    }

    #[test]
    fn test_template_error_context_with_context_lines() {
        let context = TemplateErrorContext::new("template.html", 10, None, "Error")
            .with_context_before(vec!["line 8".to_string(), "line 9".to_string()])
            .with_source_line("line 10 with error")
            .with_context_after(vec!["line 11".to_string()]);

        let formatted = context.format();
        assert!(formatted.contains("line 8"));
        assert!(formatted.contains("line 9"));
        assert!(formatted.contains("line 10 with error"));
        assert!(formatted.contains("line 11"));
    }
}

// ============================================================================
// Extended error reporting features
// ============================================================================

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Warning - template may still render
    Warning,
    /// Error - template cannot render
    Error,
    /// Critical - system-level issue
    Critical,
}

impl ErrorSeverity {
    /// Get the severity as a string
    pub fn as_str(&self) -> &str {
        match self {
            ErrorSeverity::Warning => "WARNING",
            ErrorSeverity::Error => "ERROR",
            ErrorSeverity::Critical => "CRITICAL",
        }
    }
}

/// Enhanced template error with severity and additional context
#[derive(Debug, Clone)]
pub struct EnhancedError {
    /// Error severity
    pub severity: ErrorSeverity,
    /// Error context
    pub context: TemplateErrorContext,
    /// Stack trace of template includes
    pub stack_trace: Vec<String>,
    /// Related errors
    pub related_errors: Vec<String>,
}

impl EnhancedError {
    /// Create a new enhanced error
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::{EnhancedError, ErrorSeverity, TemplateErrorContext};
    ///
    /// let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
    /// let error = EnhancedError::new(ErrorSeverity::Error, context);
    /// assert_eq!(error.severity, ErrorSeverity::Error);
    /// ```
    pub fn new(severity: ErrorSeverity, context: TemplateErrorContext) -> Self {
        Self {
            severity,
            context,
            stack_trace: Vec::new(),
            related_errors: Vec::new(),
        }
    }

    /// Add a template to the stack trace
    pub fn with_stack_trace(mut self, templates: Vec<String>) -> Self {
        self.stack_trace = templates;
        self
    }

    /// Add related errors
    pub fn with_related_errors(mut self, errors: Vec<String>) -> Self {
        self.related_errors = errors;
        self
    }

    /// Format the error for display
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::{EnhancedError, ErrorSeverity, TemplateErrorContext};
    ///
    /// let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
    /// let error = EnhancedError::new(ErrorSeverity::Error, context);
    /// let formatted = error.format();
    /// assert!(formatted.contains("ERROR"));
    /// ```
    pub fn format(&self) -> String {
        let mut output = format!("[{}] ", self.severity.as_str());
        output.push_str(&self.context.format());

        if !self.stack_trace.is_empty() {
            output.push_str("\nStack trace:\n");
            for (i, template) in self.stack_trace.iter().enumerate() {
                output.push_str(&format!("  {} {}\n", i + 1, template));
            }
        }

        if !self.related_errors.is_empty() {
            output.push_str("\nRelated errors:\n");
            for error in &self.related_errors {
                output.push_str(&format!("  - {}\n", error));
            }
        }

        output
    }
}

impl std::fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Error reporter with aggregation capabilities
pub struct ErrorReporter {
    errors: Vec<EnhancedError>,
    warnings: Vec<EnhancedError>,
}

impl ErrorReporter {
    /// Create a new error reporter
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::ErrorReporter;
    ///
    /// let reporter = ErrorReporter::new();
    /// assert!(!reporter.has_errors());
    /// ```
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: EnhancedError) {
        match error.severity {
            ErrorSeverity::Warning => self.warnings.push(error),
            ErrorSeverity::Error | ErrorSeverity::Critical => self.errors.push(error),
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Get a summary report
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::{ErrorReporter, EnhancedError, ErrorSeverity, TemplateErrorContext};
    ///
    /// let mut reporter = ErrorReporter::new();
    /// let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
    /// reporter.add_error(EnhancedError::new(ErrorSeverity::Error, context));
    ///
    /// let summary = reporter.summary();
    /// assert!(summary.contains("Errors: 1"));
    /// ```
    pub fn summary(&self) -> String {
        let mut output = String::from("=== Error Report ===\n\n");

        if self.has_errors() {
            output.push_str(&format!("Errors: {}\n", self.errors.len()));
            for (i, error) in self.errors.iter().enumerate() {
                output.push_str(&format!("\n{}. {}\n", i + 1, error.format()));
            }
        }

        if self.has_warnings() {
            output.push_str(&format!("\nWarnings: {}\n", self.warnings.len()));
            for (i, warning) in self.warnings.iter().enumerate() {
                output.push_str(&format!("\n{}. {}\n", i + 1, warning.format()));
            }
        }

        if !self.has_errors() && !self.has_warnings() {
            output.push_str("No errors or warnings\n");
        }

        output
    }

    /// Clear all errors and warnings
    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn test_error_severity_as_str() {
        assert_eq!(ErrorSeverity::Warning.as_str(), "WARNING");
        assert_eq!(ErrorSeverity::Error.as_str(), "ERROR");
        assert_eq!(ErrorSeverity::Critical.as_str(), "CRITICAL");
    }

    #[test]
    fn test_enhanced_error_new() {
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        let error = EnhancedError::new(ErrorSeverity::Error, context);

        assert_eq!(error.severity, ErrorSeverity::Error);
        assert!(error.stack_trace.is_empty());
        assert!(error.related_errors.is_empty());
    }

    #[test]
    fn test_enhanced_error_with_stack_trace() {
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        let error = EnhancedError::new(ErrorSeverity::Error, context)
            .with_stack_trace(vec!["base.html".to_string(), "test.html".to_string()]);

        assert_eq!(error.stack_trace.len(), 2);
    }

    #[test]
    fn test_enhanced_error_format() {
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        let error = EnhancedError::new(ErrorSeverity::Error, context);

        let formatted = error.format();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("test.html"));
    }

    #[test]
    fn test_error_reporter_new() {
        let reporter = ErrorReporter::new();
        assert!(!reporter.has_errors());
        assert!(!reporter.has_warnings());
    }

    #[test]
    fn test_error_reporter_add_error() {
        let mut reporter = ErrorReporter::new();
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        reporter.add_error(EnhancedError::new(ErrorSeverity::Error, context));

        assert!(reporter.has_errors());
        assert_eq!(reporter.error_count(), 1);
    }

    #[test]
    fn test_error_reporter_add_warning() {
        let mut reporter = ErrorReporter::new();
        let context = TemplateErrorContext::new("test.html", 10, None, "Test warning");
        reporter.add_error(EnhancedError::new(ErrorSeverity::Warning, context));

        assert!(reporter.has_warnings());
        assert_eq!(reporter.warning_count(), 1);
    }

    #[test]
    fn test_error_reporter_summary() {
        let mut reporter = ErrorReporter::new();
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        reporter.add_error(EnhancedError::new(ErrorSeverity::Error, context));

        let summary = reporter.summary();
        assert!(summary.contains("Errors: 1"));
    }

    #[test]
    fn test_error_reporter_clear() {
        let mut reporter = ErrorReporter::new();
        let context = TemplateErrorContext::new("test.html", 10, None, "Test error");
        reporter.add_error(EnhancedError::new(ErrorSeverity::Error, context));

        assert!(reporter.has_errors());
        reporter.clear();
        assert!(!reporter.has_errors());
    }
}
