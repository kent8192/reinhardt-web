use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Required,
    Invalid,
    MinLength,
    MaxLength,
    MinValue,
    MaxValue,
    Custom(String),
}

#[derive(Debug, thiserror::Error)]
pub enum FieldError {
    #[error("{0}")]
    Required(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Validation(String),
}

pub type FieldResult<T> = Result<T, FieldError>;

impl FieldError {
    /// Creates a required field error
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::FieldError;
    ///
    /// let error = FieldError::required(None);
    /// assert_eq!(error.to_string(), "This field is required.");
    ///
    /// let custom_error = FieldError::required(Some("Name is mandatory"));
    /// assert_eq!(custom_error.to_string(), "Name is mandatory");
    /// ```
    pub fn required(custom_msg: Option<&str>) -> Self {
        FieldError::Required(custom_msg.unwrap_or("This field is required.").to_string())
    }
    /// Creates an invalid field error
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::FieldError;
    ///
    /// let error = FieldError::invalid(None, "Invalid input format");
    /// assert_eq!(error.to_string(), "Invalid input format");
    ///
    /// let custom_error = FieldError::invalid(Some("Must be a number"), "Invalid");
    /// assert_eq!(custom_error.to_string(), "Must be a number");
    /// ```
    pub fn invalid(custom_msg: Option<&str>, default_msg: &str) -> Self {
        FieldError::Invalid(custom_msg.unwrap_or(default_msg).to_string())
    }
    /// Creates a validation field error
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::FieldError;
    ///
    /// let error = FieldError::validation(None, "Value out of range");
    /// assert_eq!(error.to_string(), "Value out of range");
    ///
    /// let custom_error = FieldError::validation(Some("Too long"), "Length exceeded");
    /// assert_eq!(custom_error.to_string(), "Too long");
    /// ```
    pub fn validation(custom_msg: Option<&str>, default_msg: &str) -> Self {
        FieldError::Validation(custom_msg.unwrap_or(default_msg).to_string())
    }
}

/// Field widget type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Widget {
    TextInput,
    PasswordInput,
    EmailInput,
    NumberInput,
    TextArea,
    Select { choices: Vec<(String, String)> },
    CheckboxInput,
    RadioSelect { choices: Vec<(String, String)> },
    DateInput,
    DateTimeInput,
    FileInput,
    HiddenInput,
}

impl Widget {
    /// Renders the widget as HTML
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::Widget;
    ///
    /// let widget = Widget::TextInput;
    /// let html = widget.render_html("username", Some("john_doe"), None);
    /// assert!(html.contains("<input"));
    /// assert!(html.contains("type=\"text\""));
    /// assert!(html.contains("name=\"username\""));
    /// assert!(html.contains("value=\"john_doe\""));
    /// ```
    pub fn render_html(
        &self,
        name: &str,
        value: Option<&str>,
        attrs: Option<&HashMap<String, String>>,
    ) -> String {
        let mut html = String::new();
        let default_attrs = HashMap::new();
        let attrs = attrs.unwrap_or(&default_attrs);

        // Build common attributes
        let mut common_attrs = String::new();
        for (key, val) in attrs {
            common_attrs.push_str(&format!(" {}=\"{}\"", key, val));
        }

        match self {
            Widget::TextInput => {
                html.push_str(&format!(
                    "<input type=\"text\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::PasswordInput => {
                html.push_str(&format!(
                    "<input type=\"password\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::EmailInput => {
                html.push_str(&format!(
                    "<input type=\"email\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::NumberInput => {
                html.push_str(&format!(
                    "<input type=\"number\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::TextArea => {
                html.push_str(&format!("<textarea name=\"{}\"{}", name, common_attrs));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(">");
                html.push_str(value.unwrap_or(""));
                html.push_str("</textarea>");
            }
            Widget::Select { choices } => {
                html.push_str(&format!("<select name=\"{}\"{}", name, common_attrs));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(">");
                for (choice_value, choice_label) in choices {
                    let selected = if Some(choice_value.as_str()) == value {
                        " selected"
                    } else {
                        ""
                    };
                    html.push_str(&format!(
                        "<option value=\"{}\"{}>{}</option>",
                        choice_value, selected, choice_label
                    ));
                }
                html.push_str("</select>");
            }
            Widget::CheckboxInput => {
                html.push_str(&format!("<input type=\"checkbox\" name=\"{}\"", name));
                if value == Some("true") || value == Some("on") {
                    html.push_str(" checked");
                }
                html.push_str(&common_attrs);
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::RadioSelect { choices } => {
                for (i, (choice_value, choice_label)) in choices.iter().enumerate() {
                    let checked = if Some(choice_value.as_str()) == value {
                        " checked"
                    } else {
                        ""
                    };
                    html.push_str(&format!(
                        "<input type=\"radio\" name=\"{}\" value=\"{}\" id=\"id_{}_{}\"{}{} />",
                        name, choice_value, name, i, checked, common_attrs
                    ));
                    html.push_str(&format!(
                        "<label for=\"id_{}_{}\">{}</label>",
                        name, i, choice_label
                    ));
                }
            }
            Widget::DateInput => {
                html.push_str(&format!(
                    "<input type=\"date\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::DateTimeInput => {
                html.push_str(&format!(
                    "<input type=\"datetime-local\" name=\"{}\" value=\"{}\"{}",
                    name,
                    value.unwrap_or(""),
                    common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::FileInput => {
                html.push_str(&format!(
                    "<input type=\"file\" name=\"{}\"{}",
                    name, common_attrs
                ));
                if !attrs.contains_key("id") {
                    html.push_str(&format!(" id=\"id_{}\"", name));
                }
                html.push_str(" />");
            }
            Widget::HiddenInput => {
                html.push_str(&format!(
                    "<input type=\"hidden\" name=\"{}\" value=\"{}\" />",
                    name,
                    value.unwrap_or("")
                ));
            }
        }

        html
    }
}

/// Base field trait for forms
///
/// This trait is specifically for form fields. For ORM fields, use `reinhardt_orm::Field`.
pub trait FormField: Send + Sync {
    fn name(&self) -> &str;
    fn label(&self) -> Option<&str>;
    fn required(&self) -> bool;
    fn help_text(&self) -> Option<&str>;
    fn widget(&self) -> &Widget;
    fn initial(&self) -> Option<&serde_json::Value>;

    fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value>;

    /// Check if the field value has changed from its initial value
    fn has_changed(
        &self,
        initial: Option<&serde_json::Value>,
        data: Option<&serde_json::Value>,
    ) -> bool {
        // Default implementation: compare values directly
        match (initial, data) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(i), Some(d)) => i != d,
        }
    }

    /// Get custom error messages for this field
    fn error_messages(&self) -> HashMap<ErrorType, String> {
        HashMap::new()
    }
}

/// CharField for text input
pub struct CharField {
    pub name: String,
    pub label: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
    pub widget: Widget,
    pub initial: Option<serde_json::Value>,
    pub max_length: Option<usize>,
    pub min_length: Option<usize>,
    pub strip: bool,                 // Whether to strip whitespace (default: true)
    pub empty_value: Option<String>, // What to return for empty values (default: empty string)
}

impl CharField {
    /// Documentation for `new`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{CharField, Field};
    ///
    /// let field = CharField::new("username".to_string());
    /// assert_eq!(field.name(), "username");
    /// assert!(field.required());
    ///
    // Field can clean and validate input
    /// let result = field.clean(Some(&serde_json::json!("john_doe")));
    /// assert!(result.is_ok());
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            label: None,
            required: true,
            help_text: None,
            widget: Widget::TextInput,
            initial: None,
            max_length: None,
            min_length: None,
            strip: true,
            empty_value: Some(String::new()),
        }
    }
}

impl FormField for CharField {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn required(&self) -> bool {
        self.required
    }

    fn help_text(&self) -> Option<&str> {
        self.help_text.as_deref()
    }

    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn initial(&self) -> Option<&serde_json::Value> {
        self.initial.as_ref()
    }

    fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
        match value {
            None if self.required => Err(FieldError::required(None)),
            None => {
                if let Some(ref empty) = self.empty_value {
                    Ok(serde_json::Value::String(empty.clone()))
                } else {
                    Ok(serde_json::Value::Null)
                }
            }
            Some(v) => {
                // Convert value to string (Django-like behavior)
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Array(arr) => {
                        // Convert array to string representation
                        serde_json::to_string(arr).unwrap_or_else(|_| String::from("[]"))
                    }
                    serde_json::Value::Null => String::new(),
                    _ => {
                        return Err(FieldError::Invalid(
                            "Cannot convert value to string".to_string(),
                        ));
                    }
                };

                // Strip whitespace if enabled
                let s = if self.strip { s.trim() } else { &s };

                // Check for null characters
                if s.contains('\0') {
                    return Err(FieldError::Validation(
                        "Null characters are not allowed".to_string(),
                    ));
                }

                // Handle empty string
                if s.is_empty() {
                    if self.required {
                        return Err(FieldError::required(None));
                    }
                    return Ok(serde_json::Value::String(
                        self.empty_value.clone().unwrap_or_default(),
                    ));
                }

                // Validate length constraints
                if let Some(max) = self.max_length {
                    if s.len() > max {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value has at most {} characters (it has {})",
                            max,
                            s.len()
                        )));
                    }
                }

                if let Some(min) = self.min_length {
                    if s.len() < min {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value has at least {} characters (it has {})",
                            min,
                            s.len()
                        )));
                    }
                }

                Ok(serde_json::Value::String(s.to_string()))
            }
        }
    }
}

/// IntegerField for integer input
pub struct IntegerField {
    pub name: String,
    pub label: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
    pub widget: Widget,
    pub initial: Option<serde_json::Value>,
    pub max_value: Option<i64>,
    pub min_value: Option<i64>,
}

impl IntegerField {
    /// Documentation for `new`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{IntegerField, Field};
    ///
    /// let field = IntegerField::new("age".to_string());
    /// assert_eq!(field.name(), "age");
    /// assert!(field.required());
    ///
    // Field can clean and validate integer input
    /// let result = field.clean(Some(&serde_json::json!(25)));
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), serde_json::json!(25));
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            label: None,
            required: true,
            help_text: None,
            widget: Widget::NumberInput,
            initial: None,
            max_value: None,
            min_value: None,
        }
    }
}

impl FormField for IntegerField {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn required(&self) -> bool {
        self.required
    }

    fn help_text(&self) -> Option<&str> {
        self.help_text.as_deref()
    }

    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn initial(&self) -> Option<&serde_json::Value> {
        self.initial.as_ref()
    }

    fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
        match value {
            None if self.required => Err(FieldError::required(None)),
            None => Ok(serde_json::Value::Null),
            Some(v) => {
                // Parse integer from either number or string
                let num = if let Some(n) = v.as_i64() {
                    n
                } else if let Some(s) = v.as_str() {
                    // Trim whitespace
                    let s = s.trim();

                    // Return None/error for empty string
                    if s.is_empty() {
                        if self.required {
                            return Err(FieldError::required(None));
                        }
                        return Ok(serde_json::Value::Null);
                    }

                    // Parse string to integer
                    s.parse::<i64>()
                        .map_err(|_| FieldError::Invalid("Enter a whole number".to_string()))?
                } else {
                    return Err(FieldError::Invalid(
                        "Expected integer or string".to_string(),
                    ));
                };

                // Validate range
                if let Some(max) = self.max_value {
                    if num > max {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value is less than or equal to {}",
                            max
                        )));
                    }
                }

                if let Some(min) = self.min_value {
                    if num < min {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value is greater than or equal to {}",
                            min
                        )));
                    }
                }

                Ok(serde_json::Value::Number(num.into()))
            }
        }
    }
}

/// BooleanField for checkbox input
pub struct BooleanField {
    pub name: String,
    pub label: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
    pub initial: Option<serde_json::Value>,
}

impl BooleanField {
    /// Documentation for `new`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BooleanField, Field};
    ///
    /// let field = BooleanField::new("accept_terms".to_string());
    /// assert_eq!(field.name(), "accept_terms");
    /// assert!(!field.required()); // BooleanField is not required by default
    ///
    // Field can clean and validate boolean input
    /// let result = field.clean(Some(&serde_json::json!(true)));
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), serde_json::json!(true));
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            label: None,
            required: false,
            help_text: None,
            initial: None,
        }
    }
}

impl FormField for BooleanField {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn required(&self) -> bool {
        self.required
    }

    fn help_text(&self) -> Option<&str> {
        self.help_text.as_deref()
    }

    fn widget(&self) -> &Widget {
        &Widget::CheckboxInput
    }

    fn initial(&self) -> Option<&serde_json::Value> {
        self.initial.as_ref()
    }

    fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
        match value {
            None => {
                if self.required {
                    Err(FieldError::required(None))
                } else {
                    Ok(serde_json::Value::Bool(false))
                }
            }
            Some(v) => {
                // Convert various types to boolean (Django-like behavior)
                let b = match v {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::String(s) => {
                        // String conversion: "false", "False", "0", "" -> false, others -> true
                        let s_lower = s.to_lowercase();
                        !(s.is_empty() || s_lower == "false" || s == "0")
                    }
                    serde_json::Value::Number(n) => {
                        // Numbers: 0 -> false, non-zero -> true
                        if let Some(i) = n.as_i64() {
                            i != 0
                        } else if let Some(f) = n.as_f64() {
                            f != 0.0
                        } else {
                            true
                        }
                    }
                    serde_json::Value::Null => false,
                    _ => return Err(FieldError::Invalid("Cannot convert to boolean".to_string())),
                };

                // Check required constraint (for BooleanField, required means it must be true)
                if self.required && !b {
                    return Err(FieldError::required(None));
                }

                Ok(serde_json::Value::Bool(b))
            }
        }
    }
}

/// EmailField for email address input
pub struct EmailField {
    pub name: String,
    pub label: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
    pub widget: Widget,
    pub initial: Option<serde_json::Value>,
    pub max_length: Option<usize>,
    pub min_length: Option<usize>,
}

impl EmailField {
    /// Documentation for `new`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{EmailField, Field};
    ///
    /// let field = EmailField::new("email".to_string());
    /// assert_eq!(field.name(), "email");
    /// assert!(field.required());
    ///
    // Field validates email format
    /// let valid_result = field.clean(Some(&serde_json::json!("user@example.com")));
    /// assert!(valid_result.is_ok());
    ///
    /// let invalid_result = field.clean(Some(&serde_json::json!("not-an-email")));
    /// assert!(invalid_result.is_err());
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            label: None,
            required: true,
            help_text: None,
            widget: Widget::EmailInput,
            initial: None,
            max_length: Some(320), // RFC standard: 64 (local) + @ + 255 (domain)
            min_length: None,
        }
    }

    fn validate_email(email: &str) -> bool {
        // Basic email validation regex
        // This is a simplified version - production should use a more robust validator
        let email_regex = regex::Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).unwrap();

        email_regex.is_match(email)
    }
}

impl FormField for EmailField {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn required(&self) -> bool {
        self.required
    }

    fn help_text(&self) -> Option<&str> {
        self.help_text.as_deref()
    }

    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn initial(&self) -> Option<&serde_json::Value> {
        self.initial.as_ref()
    }

    fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
        match value {
            None if self.required => Err(FieldError::required(None)),
            None => Ok(serde_json::Value::String(String::new())),
            Some(v) => {
                let s = v
                    .as_str()
                    .ok_or_else(|| FieldError::Invalid("Expected string".to_string()))?;

                // Trim whitespace
                let s = s.trim();

                // Return empty string if not required and empty
                if s.is_empty() {
                    if self.required {
                        return Err(FieldError::required(None));
                    }
                    return Ok(serde_json::Value::String(String::new()));
                }

                // Check length constraints
                if let Some(max) = self.max_length {
                    if s.len() > max {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value has at most {} characters (it has {})",
                            max,
                            s.len()
                        )));
                    }
                }

                if let Some(min) = self.min_length {
                    if s.len() < min {
                        return Err(FieldError::Validation(format!(
                            "Ensure this value has at least {} characters (it has {})",
                            min,
                            s.len()
                        )));
                    }
                }

                // Validate email format
                if !Self::validate_email(s) {
                    return Err(FieldError::Validation(
                        "Enter a valid email address".to_string(),
                    ));
                }

                Ok(serde_json::Value::String(s.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forms_char_field_validation() {
        let mut field = CharField::new("name".to_string());
        field.max_length = Some(10);
        field.min_length = Some(2);

        // Valid value
        let result = field.clean(Some(&serde_json::json!("John")));
        assert!(result.is_ok());

        // Too long
        let result = field.clean(Some(&serde_json::json!("VeryLongName")));
        assert!(result.is_err());

        // Too short
        let result = field.clean(Some(&serde_json::json!("J")));
        assert!(result.is_err());
    }

    #[test]
    fn test_forms_integer_field_validation() {
        let mut field = IntegerField::new("age".to_string());
        field.max_value = Some(100);
        field.min_value = Some(0);

        // Valid value
        let result = field.clean(Some(&serde_json::json!(25)));
        assert!(result.is_ok());

        // Too large
        let result = field.clean(Some(&serde_json::json!(150)));
        assert!(result.is_err());

        // Too small
        let result = field.clean(Some(&serde_json::json!(-5)));
        assert!(result.is_err());
    }

    #[test]
    fn test_forms_field_boolean() {
        let field = BooleanField::new("active".to_string());

        let result = field.clean(Some(&serde_json::json!(true)));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(true));

        let result = field.clean(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(false));
    }

    // Additional tests based on Django forms tests

    #[test]
    fn test_charfield_required() {
        // Test based on Django test_charfield_1
        let field = CharField::new("test".to_string());

        // Should accept valid strings
        assert_eq!(
            field.clean(Some(&serde_json::json!("hello"))).unwrap(),
            serde_json::json!("hello")
        );

        // Should reject None when required
        assert!(matches!(field.clean(None), Err(FieldError::Required(_))));

        // Now empty strings are properly rejected for required fields
        assert!(matches!(
            field.clean(Some(&serde_json::json!(""))),
            Err(FieldError::Required(_))
        ));
    }

    #[test]
    fn test_charfield_strip() {
        // Test whitespace stripping
        let field = CharField::new("test".to_string());

        // Whitespace should be stripped by default
        assert_eq!(
            field.clean(Some(&serde_json::json!("  hello  "))).unwrap(),
            serde_json::json!("hello")
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("\thello\n"))).unwrap(),
            serde_json::json!("hello")
        );

        // Test with strip=false
        let mut field_no_strip = CharField::new("test".to_string());
        field_no_strip.strip = false;
        assert_eq!(
            field_no_strip
                .clean(Some(&serde_json::json!("  hello  ")))
                .unwrap(),
            serde_json::json!("  hello  ")
        );
    }

    #[test]
    fn test_charfield_type_conversion() {
        // Test converting non-string values to strings
        let mut field = CharField::new("test".to_string());
        field.required = false;

        // Numbers should be converted to strings
        assert_eq!(
            field.clean(Some(&serde_json::json!(42))).unwrap(),
            serde_json::json!("42")
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(3.14))).unwrap(),
            serde_json::json!("3.14")
        );

        // Booleans should be converted
        assert_eq!(
            field.clean(Some(&serde_json::json!(true))).unwrap(),
            serde_json::json!("true")
        );

        // Arrays should be converted to JSON string
        let result = field.clean(Some(&serde_json::json!([1, 2, 3]))).unwrap();
        assert!(result.as_str().unwrap().contains("1"));
    }

    #[test]
    fn test_charfield_null_characters() {
        // Null characters should be rejected
        let field = CharField::new("test".to_string());

        let result = field.clean(Some(&serde_json::json!("hello\0world")));
        assert!(matches!(result, Err(FieldError::Validation(_))));
        if let Err(FieldError::Validation(msg)) = result {
            assert!(msg.contains("Null characters"));
        }
    }

    #[test]
    fn test_charfield_empty_whitespace() {
        // Whitespace-only values should be treated as empty after stripping
        let field = CharField::new("test".to_string());

        assert!(matches!(
            field.clean(Some(&serde_json::json!("   "))),
            Err(FieldError::Required(_))
        ));
    }

    #[test]
    fn test_charfield_not_required() {
        // Test based on Django test_charfield_2
        let mut field = CharField::new("test".to_string());
        field.required = false;

        assert_eq!(
            field.clean(Some(&serde_json::json!("hello"))).unwrap(),
            serde_json::json!("hello")
        );

        // Should return empty string when not required and None (due to empty_value)
        assert_eq!(field.clean(None).unwrap(), serde_json::json!(""));

        // Should accept empty string when not required
        assert_eq!(
            field.clean(Some(&serde_json::json!(""))).unwrap(),
            serde_json::json!("")
        );
    }

    #[test]
    fn test_charfield_max_length() {
        // Test based on Django test_charfield_3
        let mut field = CharField::new("test".to_string());
        field.required = false;
        field.max_length = Some(10);

        assert_eq!(
            field.clean(Some(&serde_json::json!("12345"))).unwrap(),
            serde_json::json!("12345")
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("1234567890"))).unwrap(),
            serde_json::json!("1234567890")
        );

        // Should reject strings longer than max_length
        let result = field.clean(Some(&serde_json::json!("1234567890a")));
        assert!(matches!(result, Err(FieldError::Validation(_))));
        if let Err(FieldError::Validation(msg)) = result {
            assert!(msg.contains("at most 10 characters"));
        }
    }

    #[test]
    fn test_charfield_min_length() {
        // Test based on Django test_charfield_4
        let mut field = CharField::new("test".to_string());
        field.required = false;
        field.min_length = Some(10);

        // Empty string is now allowed when not required (Django behavior)
        assert_eq!(
            field.clean(Some(&serde_json::json!(""))).unwrap(),
            serde_json::json!("")
        );

        // Should reject strings shorter than min_length
        let result = field.clean(Some(&serde_json::json!("12345")));
        assert!(matches!(result, Err(FieldError::Validation(_))));

        assert_eq!(
            field.clean(Some(&serde_json::json!("1234567890"))).unwrap(),
            serde_json::json!("1234567890")
        );
    }

    #[test]
    fn test_integerfield_required() {
        // Test based on Django test_integerfield_1
        let field = IntegerField::new("test".to_string());

        // Should accept valid integers
        assert_eq!(
            field.clean(Some(&serde_json::json!(1))).unwrap(),
            serde_json::json!(1)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(42))).unwrap(),
            serde_json::json!(42)
        );

        // Should reject None when required
        assert!(matches!(field.clean(None), Err(FieldError::Required(_))));
    }

    #[test]
    fn test_integerfield_not_required() {
        // Test based on Django test_integerfield_2
        let mut field = IntegerField::new("test".to_string());
        field.required = false;

        assert_eq!(
            field.clean(Some(&serde_json::json!(1))).unwrap(),
            serde_json::json!(1)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(23))).unwrap(),
            serde_json::json!(23)
        );

        // Should return Null when not required and None
        assert_eq!(field.clean(None).unwrap(), serde_json::json!(null));
    }

    #[test]
    fn test_integerfield_max_value() {
        // Test based on Django test_integerfield_3
        let mut field = IntegerField::new("test".to_string());
        field.max_value = Some(10);

        assert_eq!(
            field.clean(Some(&serde_json::json!(1))).unwrap(),
            serde_json::json!(1)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(10))).unwrap(),
            serde_json::json!(10)
        );

        // Should reject values exceeding max_value
        let result = field.clean(Some(&serde_json::json!(11)));
        assert!(matches!(result, Err(FieldError::Validation(_))));
        if let Err(FieldError::Validation(msg)) = result {
            assert!(msg.contains("less than or equal to 10"));
        }
    }

    #[test]
    fn test_integerfield_min_value() {
        // Test based on Django test_integerfield_4
        let mut field = IntegerField::new("test".to_string());
        field.min_value = Some(10);

        assert_eq!(
            field.clean(Some(&serde_json::json!(10))).unwrap(),
            serde_json::json!(10)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(11))).unwrap(),
            serde_json::json!(11)
        );

        // Should reject values below min_value
        let result = field.clean(Some(&serde_json::json!(9)));
        assert!(matches!(result, Err(FieldError::Validation(_))));
        if let Err(FieldError::Validation(msg)) = result {
            assert!(msg.contains("greater than or equal to 10"));
        }
    }

    #[test]
    fn test_integerfield_min_max_value() {
        // Test with both min and max values
        let mut field = IntegerField::new("test".to_string());
        field.min_value = Some(10);
        field.max_value = Some(20);

        assert_eq!(
            field.clean(Some(&serde_json::json!(10))).unwrap(),
            serde_json::json!(10)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(15))).unwrap(),
            serde_json::json!(15)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(20))).unwrap(),
            serde_json::json!(20)
        );

        // Below min
        assert!(matches!(
            field.clean(Some(&serde_json::json!(9))),
            Err(FieldError::Validation(_))
        ));

        // Above max
        assert!(matches!(
            field.clean(Some(&serde_json::json!(21))),
            Err(FieldError::Validation(_))
        ));
    }

    #[test]
    fn test_integerfield_negative_numbers() {
        // IntegerField should handle negative numbers
        let mut field = IntegerField::new("test".to_string());
        field.required = false;

        assert_eq!(
            field.clean(Some(&serde_json::json!(-1))).unwrap(),
            serde_json::json!(-1)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(-42))).unwrap(),
            serde_json::json!(-42)
        );
    }

    #[test]
    fn test_booleanfield_default_not_required() {
        // BooleanField is not required by default (unlike CharField/IntegerField)
        let field = BooleanField::new("test".to_string());
        assert!(!field.required);

        // None should be treated as false
        assert_eq!(field.clean(None).unwrap(), serde_json::json!(false));
    }

    // IntegerField string parsing tests

    #[test]
    fn test_integerfield_string_parsing() {
        let mut field = IntegerField::new("test".to_string());
        field.required = false;

        // Should parse string integers
        assert_eq!(
            field.clean(Some(&serde_json::json!("42"))).unwrap(),
            serde_json::json!(42)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("  123  "))).unwrap(),
            serde_json::json!(123)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("-10"))).unwrap(),
            serde_json::json!(-10)
        );

        // Should reject invalid strings
        assert!(matches!(
            field.clean(Some(&serde_json::json!("abc"))),
            Err(FieldError::Invalid(_))
        ));
        assert!(matches!(
            field.clean(Some(&serde_json::json!("12.5"))),
            Err(FieldError::Invalid(_))
        ));
    }

    #[test]
    fn test_integerfield_empty_string() {
        let mut field = IntegerField::new("test".to_string());
        field.required = false;

        // Empty string should return Null when not required
        assert_eq!(
            field.clean(Some(&serde_json::json!(""))).unwrap(),
            serde_json::json!(null)
        );

        // But should error when required
        field.required = true;
        assert!(matches!(
            field.clean(Some(&serde_json::json!(""))),
            Err(FieldError::Required(_))
        ));
    }

    // BooleanField string conversion tests

    #[test]
    fn test_booleanfield_string_conversion() {
        let field = BooleanField::new("test".to_string());

        // String "false" variations should be false
        assert_eq!(
            field.clean(Some(&serde_json::json!("false"))).unwrap(),
            serde_json::json!(false)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("False"))).unwrap(),
            serde_json::json!(false)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("FALSE"))).unwrap(),
            serde_json::json!(false)
        );

        // String "0" and empty should be false
        assert_eq!(
            field.clean(Some(&serde_json::json!("0"))).unwrap(),
            serde_json::json!(false)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(""))).unwrap(),
            serde_json::json!(false)
        );

        // Other strings should be true
        assert_eq!(
            field.clean(Some(&serde_json::json!("true"))).unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("1"))).unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!("yes"))).unwrap(),
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_booleanfield_number_conversion() {
        let field = BooleanField::new("test".to_string());

        // 0 should be false
        assert_eq!(
            field.clean(Some(&serde_json::json!(0))).unwrap(),
            serde_json::json!(false)
        );

        // Non-zero should be true
        assert_eq!(
            field.clean(Some(&serde_json::json!(1))).unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(-1))).unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(
            field.clean(Some(&serde_json::json!(42))).unwrap(),
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_booleanfield_required() {
        let mut field = BooleanField::new("test".to_string());
        field.required = true;

        // Required BooleanField must be true
        assert_eq!(
            field.clean(Some(&serde_json::json!(true))).unwrap(),
            serde_json::json!(true)
        );

        // False should be rejected
        assert!(matches!(
            field.clean(Some(&serde_json::json!(false))),
            Err(FieldError::Required(_))
        ));
        assert!(matches!(field.clean(None), Err(FieldError::Required(_))));
    }

    // EmailField tests

    #[test]
    fn test_emailfield_required() {
        let field = EmailField::new("email".to_string());

        // Valid email addresses
        assert_eq!(
            field
                .clean(Some(&serde_json::json!("user@example.com")))
                .unwrap(),
            serde_json::json!("user@example.com")
        );

        // Required field rejects None and empty
        assert!(matches!(field.clean(None), Err(FieldError::Required(_))));
        assert!(matches!(
            field.clean(Some(&serde_json::json!(""))),
            Err(FieldError::Required(_))
        ));

        // Invalid email format
        assert!(matches!(
            field.clean(Some(&serde_json::json!("invalid"))),
            Err(FieldError::Validation(_))
        ));
    }

    #[test]
    fn test_emailfield_not_required() {
        let mut field = EmailField::new("email".to_string());
        field.required = false;

        // Empty values return empty string
        assert_eq!(field.clean(None).unwrap(), serde_json::json!(""));
        assert_eq!(
            field.clean(Some(&serde_json::json!(""))).unwrap(),
            serde_json::json!("")
        );

        // Valid emails
        assert_eq!(
            field
                .clean(Some(&serde_json::json!("test@test.com")))
                .unwrap(),
            serde_json::json!("test@test.com")
        );

        // Whitespace trimming
        assert_eq!(
            field
                .clean(Some(&serde_json::json!("  user@example.com  ")))
                .unwrap(),
            serde_json::json!("user@example.com")
        );
    }

    #[test]
    fn test_emailfield_validation() {
        let field = EmailField::new("email".to_string());

        // Valid formats
        let valid_emails = vec![
            "user@example.com",
            "test.user@example.com",
            "user+tag@example.co.uk",
            "user_name@example-domain.com",
        ];

        for email in valid_emails {
            assert!(
                field.clean(Some(&serde_json::json!(email))).is_ok(),
                "Failed to validate: {}",
                email
            );
        }

        // Invalid formats
        let invalid_emails = vec![
            "invalid",
            "@example.com",
            "user@",
            "user @example.com",
            "user@@example.com",
        ];

        for email in invalid_emails {
            assert!(
                matches!(
                    field.clean(Some(&serde_json::json!(email))),
                    Err(FieldError::Validation(_))
                ),
                "Should reject: {}",
                email
            );
        }
    }

    #[test]
    fn test_emailfield_max_length() {
        let field = EmailField::new("email".to_string());

        // Default max_length is 320
        assert_eq!(field.max_length, Some(320));

        // Test length validation
        let mut short_field = EmailField::new("email".to_string());
        short_field.max_length = Some(15);

        assert!(short_field
            .clean(Some(&serde_json::json!("a@foo.com")))
            .is_ok());
        assert!(matches!(
            short_field.clean(Some(&serde_json::json!("verylongemail@example.com"))),
            Err(FieldError::Validation(_))
        ));
    }

    #[test]
    fn test_field_has_changed() {
        let field = CharField::new("name".to_string());

        // No change: both None
        assert!(!field.has_changed(None, None));

        // Change: initial None, data Some
        assert!(field.has_changed(None, Some(&serde_json::json!("John"))));

        // Change: initial Some, data None
        assert!(field.has_changed(Some(&serde_json::json!("John")), None));

        // No change: same value
        assert!(!field.has_changed(
            Some(&serde_json::json!("John")),
            Some(&serde_json::json!("John"))
        ));

        // Change: different value
        assert!(field.has_changed(
            Some(&serde_json::json!("John")),
            Some(&serde_json::json!("Jane"))
        ));
    }

    #[test]
    fn test_field_error_messages() {
        let field = CharField::new("name".to_string());

        // Default implementation returns empty HashMap
        assert!(field.error_messages().is_empty());
    }
}
