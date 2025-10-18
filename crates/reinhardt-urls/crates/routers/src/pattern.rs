use regex::Regex;
use std::collections::HashMap;

/// Path pattern for URL matching
/// Similar to Django's URL patterns but using composition
pub struct PathPattern {
    pattern: String,
    regex: Regex,
    param_names: Vec<String>,
}

impl PathPattern {
    /// Create a new path pattern
    /// Patterns like "/users/{id}/" are converted to regex
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{PathPattern, path};
    ///
    /// // Create a simple pattern without parameters
    /// let pattern = PathPattern::new(path!("/users/")).unwrap();
    /// assert_eq!(pattern.pattern(), "/users/");
    ///
    /// // Create a pattern with a parameter
    /// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
    /// assert_eq!(pattern.param_names(), &["id"]);
    /// ```
    pub fn new(pattern: impl Into<String>) -> Result<Self, String> {
        let pattern = pattern.into();
        let (regex_str, param_names) = Self::parse_pattern(&pattern)?;
        let regex = Regex::new(&regex_str).map_err(|e| e.to_string())?;

        Ok(Self {
            pattern,
            regex,
            param_names,
        })
    }

    fn parse_pattern(pattern: &str) -> Result<(String, Vec<String>), String> {
        let mut regex_str = String::from("^");
        let mut param_names = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    // Extract parameter name
                    let mut param_name = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        param_name.push(chars.next().unwrap());
                    }

                    if param_name.is_empty() {
                        return Err("Empty parameter name".to_string());
                    }

                    param_names.push(param_name.clone());
                    // Match any non-slash characters
                    regex_str.push_str(&format!("(?P<{}>", param_name));
                    regex_str.push_str("[^/]+)");
                }
                _ => {
                    // Escape special regex characters
                    if ".*+?^${}()|[]\\".contains(ch) {
                        regex_str.push('\\');
                    }
                    regex_str.push(ch);
                }
            }
        }

        regex_str.push('$');
        Ok((regex_str, param_names))
    }
    /// Get the original pattern string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{PathPattern, path};
    ///
    /// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
    /// assert_eq!(pattern.pattern(), "/users/{id}/");
    /// ```
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
    /// Get the list of parameter names in the pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{PathPattern, path};
    ///
    /// let pattern = PathPattern::new(path!("/users/{user_id}/posts/{post_id}/")).unwrap();
    /// assert_eq!(pattern.param_names(), &["user_id", "post_id"]);
    /// ```
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }
}

/// Path matcher - uses composition to match paths
pub struct PathMatcher {
    patterns: Vec<(PathPattern, String)>, // (pattern, handler_id)
}

impl PathMatcher {
    /// Create a new PathMatcher
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::PathMatcher;
    ///
    /// let matcher = PathMatcher::new();
    /// assert_eq!(matcher.match_path("/users/"), None);
    /// ```
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }
    /// Add a pattern to the matcher
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{PathMatcher, PathPattern, path};
    ///
    /// let mut matcher = PathMatcher::new();
    /// let pattern = PathPattern::new(path!("/users/")).unwrap();
    /// matcher.add_pattern(pattern, "users_list".to_string());
    ///
    /// let result = matcher.match_path("/users/");
    /// assert!(result.is_some());
    /// ```
    pub fn add_pattern(&mut self, pattern: PathPattern, handler_id: String) {
        self.patterns.push((pattern, handler_id));
    }
    /// Match a path and extract parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{PathMatcher, PathPattern, path};
    ///
    /// let mut matcher = PathMatcher::new();
    /// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
    /// matcher.add_pattern(pattern, "users_detail".to_string());
    ///
    /// let result = matcher.match_path("/users/123/");
    /// assert!(result.is_some());
    /// let (handler_id, params) = result.unwrap();
    /// assert_eq!(handler_id, "users_detail");
    /// assert_eq!(params.get("id"), Some(&"123".to_string()));
    /// ```
    pub fn match_path(&self, path: &str) -> Option<(String, HashMap<String, String>)> {
        for (pattern, handler_id) in &self.patterns {
            if let Some(captures) = pattern.regex.captures(path) {
                let mut params = HashMap::new();

                for name in pattern.param_names() {
                    if let Some(value) = captures.name(name) {
                        params.insert(name.clone(), value.as_str().to_string());
                    }
                }

                return Some((handler_id.clone(), params));
            }
        }

        None
    }
}

impl Default for PathMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pattern() {
        let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap();
        assert!(pattern.regex.is_match("/users/"));
        assert!(!pattern.regex.is_match("/users/123/"));
    }

    #[test]
    fn test_parameter_pattern() {
        let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
        assert_eq!(pattern.param_names(), &["id"]);
        assert!(pattern.regex.is_match("/users/123/"));
        assert!(!pattern.regex.is_match("/users/"));
    }

    #[test]
    fn test_pattern_multiple_parameters() {
        let pattern = PathPattern::new(reinhardt_routers_macros::path!(
            "/users/{user_id}/posts/{post_id}/"
        ))
        .unwrap();
        assert_eq!(pattern.param_names(), &["user_id", "post_id"]);
        assert!(pattern.regex.is_match("/users/123/posts/456/"));
    }

    #[test]
    fn test_path_matcher() {
        let mut matcher = PathMatcher::new();
        matcher.add_pattern(
            PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap(),
            "users_list".to_string(),
        );
        matcher.add_pattern(
            PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap(),
            "users_detail".to_string(),
        );

        let result = matcher.match_path("/users/123/");
        assert!(result.is_some());
        let (handler_id, params) = result.unwrap();
        assert_eq!(handler_id, "users_detail");
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }
}
