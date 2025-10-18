//! XSS prevention utilities
/// Escape HTML special characters
///
/// # Examples
///
/// ```
/// use reinhardt_security::escape_html;
///
/// let input = "<script>alert('XSS')</script>";
/// let escaped = escape_html(input);
/// assert_eq!(escaped, "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;");
///
/// let with_quotes = r#"<a href="javascript:alert('xss')">Click</a>"#;
/// let escaped_quotes = escape_html(with_quotes);
/// assert_eq!(escaped_quotes, "&lt;a href=&quot;javascript:alert(&#x27;xss&#x27;)&quot;&gt;Click&lt;/a&gt;");
/// ```
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
/// Sanitize HTML (basic implementation)
///
/// # Examples
///
/// ```
/// use reinhardt_security::sanitize_html;
///
/// let dangerous = "<script>alert('XSS')</script><b>Bold text</b>";
/// let sanitized = sanitize_html(dangerous);
/// assert_eq!(sanitized, "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;&lt;b&gt;Bold text&lt;/b&gt;");
///
/// let user_input = "User's comment with <img src=x onerror=alert(1)>";
/// let safe_output = sanitize_html(user_input);
/// assert!(safe_output.contains("&lt;img"));
/// ```
pub fn sanitize_html(input: &str) -> String {
    // Basic sanitization - in production use a proper HTML sanitizer
    escape_html(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }
}
