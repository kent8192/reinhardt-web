use crate::field::{FormField, Widget};

/// BoundField represents a field bound to form data
pub struct BoundField<'a> {
    #[allow(dead_code)]
    form_name: String,
    field: &'a Box<dyn FormField>,
    data: Option<&'a serde_json::Value>,
    errors: &'a [String],
    prefix: &'a str,
}

impl<'a> BoundField<'a> {
    /// Documentation for `new`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
    /// let data = serde_json::json!("John");
    /// let errors = vec![];
    ///
    /// let bound = BoundField::new("my_form".to_string(), &field, Some(&data), &errors, "");
    /// assert_eq!(bound.name(), "name");
    /// assert_eq!(bound.value(), Some(&data));
    /// ```
    pub fn new(
        form_name: String,
        field: &'a Box<dyn FormField>,
        data: Option<&'a serde_json::Value>,
        errors: &'a [String],
        prefix: &'a str,
    ) -> Self {
        Self {
            form_name,
            field,
            data,
            errors,
            prefix,
        }
    }
    /// Get the field name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("email".to_string()));
    /// let bound = BoundField::new("form".to_string(), &field, None, &[], "");
    /// assert_eq!(bound.name(), "email");
    /// ```
    pub fn name(&self) -> &str {
        self.field.name()
    }
    /// Get the HTML name attribute (with prefix)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("email".to_string()));
    ///
    // Without prefix
    /// let bound = BoundField::new("form".to_string(), &field, None, &[], "");
    /// assert_eq!(bound.html_name(), "email");
    ///
    // With prefix
    /// let bound_prefixed = BoundField::new("form".to_string(), &field, None, &[], "user");
    /// assert_eq!(bound_prefixed.html_name(), "user-email");
    /// ```
    pub fn html_name(&self) -> String {
        if self.prefix.is_empty() {
            self.field.name().to_string()
        } else {
            format!("{}-{}", self.prefix, self.field.name())
        }
    }
    /// Get the HTML id attribute
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("username".to_string()));
    /// let bound = BoundField::new("form".to_string(), &field, None, &[], "profile");
    ///
    /// assert_eq!(bound.id_for_label(), "id_profile-username");
    /// ```
    pub fn id_for_label(&self) -> String {
        format!("id_{}", self.html_name())
    }
    /// Get the field label
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let mut field = CharField::new("name".to_string());
    /// field.label = Some("Full Name".to_string());
    /// let field_box: Box<dyn FormField> = Box::new(field);
    ///
    /// let bound = BoundField::new("form".to_string(), &field_box, None, &[], "");
    /// assert_eq!(bound.label(), Some("Full Name"));
    /// ```
    pub fn label(&self) -> Option<&str> {
        self.field.label()
    }
    /// Get the field value
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
    /// let data = serde_json::json!("Alice");
    ///
    /// let bound = BoundField::new("form".to_string(), &field, Some(&data), &[], "");
    /// assert_eq!(bound.value(), Some(&data));
    /// ```
    pub fn value(&self) -> Option<&serde_json::Value> {
        self.data.or_else(|| self.field.initial())
    }
    /// Get field errors
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("email".to_string()));
    /// let errors = vec!["Invalid email format".to_string(), "Email is required".to_string()];
    ///
    /// let bound = BoundField::new("form".to_string(), &field, None, &errors, "");
    /// assert_eq!(bound.errors().len(), 2);
    /// assert_eq!(bound.errors()[0], "Invalid email format");
    /// ```
    pub fn errors(&self) -> &[String] {
        self.errors
    }
    /// Check if field has errors
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("username".to_string()));
    ///
    // Without errors
    /// let bound_ok = BoundField::new("form".to_string(), &field, None, &[], "");
    /// assert!(!bound_ok.has_errors());
    ///
    // With errors
    /// let errors = vec!["Username is required".to_string()];
    /// let bound_err = BoundField::new("form".to_string(), &field, None, &errors, "");
    /// assert!(bound_err.has_errors());
    /// ```
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    /// Get the widget
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, EmailField, FormField, Widget};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
    /// let bound = BoundField::new("form".to_string(), &field, None, &[], "");
    /// assert!(matches!(bound.widget(), Widget::TextInput));
    ///
    /// let email_field: Box<dyn FormField> = Box::new(EmailField::new("email".to_string()));
    /// let email_bound = BoundField::new("form".to_string(), &email_field, None, &[], "");
    /// assert!(matches!(email_bound.widget(), Widget::EmailInput));
    /// ```
    pub fn widget(&self) -> &Widget {
        self.field.widget()
    }
    /// Get help text
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let mut field = CharField::new("password".to_string());
    /// field.help_text = Some("Must be at least 8 characters".to_string());
    /// let field_box: Box<dyn FormField> = Box::new(field);
    ///
    /// let bound = BoundField::new("form".to_string(), &field_box, None, &[], "");
    /// assert_eq!(bound.help_text(), Some("Must be at least 8 characters"));
    /// ```
    pub fn help_text(&self) -> Option<&str> {
        self.field.help_text()
    }
    /// Check if field is required
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let mut field = CharField::new("name".to_string());
    /// field.required = true;
    /// let field_box: Box<dyn FormField> = Box::new(field);
    ///
    /// let bound = BoundField::new("form".to_string(), &field_box, None, &[], "");
    /// assert!(bound.is_required());
    ///
    /// let mut optional_field = CharField::new("nickname".to_string());
    /// optional_field.required = false;
    /// let optional_box: Box<dyn FormField> = Box::new(optional_field);
    ///
    /// let optional_bound = BoundField::new("form".to_string(), &optional_box, None, &[], "");
    /// assert!(!optional_bound.is_required());
    /// ```
    pub fn is_required(&self) -> bool {
        self.field.required()
    }
    /// Render the field as HTML string (basic implementation)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let field: Box<dyn FormField> = Box::new(CharField::new("username".to_string()));
    /// let data = serde_json::json!("john_doe");
    ///
    /// let bound = BoundField::new("form".to_string(), &field, Some(&data), &[], "");
    /// let html = bound.as_widget();
    ///
    /// assert!(html.contains(r#"type="text""#));
    /// assert!(html.contains(r#"name="username""#));
    /// assert!(html.contains(r#"value="john_doe""#));
    /// ```
    pub fn as_widget(&self) -> String {
        let html_name = self.html_name();
        let id = self.id_for_label();
        let value_str = self
            .value()
            .and_then(|v| if v.is_string() { v.as_str() } else { None })
            .unwrap_or("");

        match self.widget() {
            Widget::TextInput => {
                format!(
                    r#"<input type="text" name="{}" id="{}" value="{}" />"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::EmailInput => {
                format!(
                    r#"<input type="email" name="{}" id="{}" value="{}" />"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::NumberInput => {
                format!(
                    r#"<input type="number" name="{}" id="{}" value="{}" />"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::PasswordInput => {
                format!(
                    r#"<input type="password" name="{}" id="{}" />"#,
                    html_name, id
                )
            }
            Widget::TextArea => {
                format!(
                    r#"<textarea name="{}" id="{}">{}</textarea>"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::CheckboxInput => {
                let checked = self.value().and_then(|v| v.as_bool()).unwrap_or(false);
                format!(
                    r#"<input type="checkbox" name="{}" id="{}" {} />"#,
                    html_name,
                    id,
                    if checked { "checked" } else { "" }
                )
            }
            Widget::Select { choices } => {
                let mut options = String::new();
                for (value, label) in choices {
                    let selected = value_str == value;
                    options.push_str(&format!(
                        r#"<option value="{}" {}>{}</option>"#,
                        html_escape(value),
                        if selected { "selected" } else { "" },
                        html_escape(label)
                    ));
                }
                format!(
                    r#"<select name="{}" id="{}">{}</select>"#,
                    html_name, id, options
                )
            }
            Widget::DateInput => {
                format!(
                    r#"<input type="date" name="{}" id="{}" value="{}" />"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::DateTimeInput => {
                format!(
                    r#"<input type="datetime-local" name="{}" id="{}" value="{}" />"#,
                    html_name,
                    id,
                    html_escape(value_str)
                )
            }
            Widget::FileInput => {
                format!(r#"<input type="file" name="{}" id="{}" />"#, html_name, id)
            }
            Widget::HiddenInput => {
                format!(
                    r#"<input type="hidden" name="{}" value="{}" />"#,
                    html_name,
                    html_escape(value_str)
                )
            }
            Widget::RadioSelect { choices } => {
                let mut radios = String::new();
                for (idx, (value, label)) in choices.iter().enumerate() {
                    let checked = value_str == value;
                    radios.push_str(&format!(
                        r#"<label><input type="radio" name="{}" id="{}_{}" value="{}" {} /> {}</label>"#,
                        html_name,
                        id,
                        idx,
                        html_escape(value),
                        if checked { "checked" } else { "" },
                        html_escape(label)
                    ));
                }
                radios
            }
        }
    }
    /// Render the field with label
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_forms::{BoundField, CharField, FormField};
    ///
    /// let mut field = CharField::new("email".to_string());
    /// field.label = Some("Email Address".to_string());
    /// field.required = true;
    /// let field_box: Box<dyn FormField> = Box::new(field);
    ///
    /// let data = serde_json::json!("test@example.com");
    /// let bound = BoundField::new("form".to_string(), &field_box, Some(&data), &[], "");
    /// let html = bound.as_field();
    ///
    /// assert!(html.contains(r#"<label for="id_email">Email Address *</label>"#));
    /// assert!(html.contains(r#"type="text""#));
    /// assert!(html.contains(r#"name="email""#));
    /// ```
    pub fn as_field(&self) -> String {
        let label = self.label().unwrap_or(self.name());
        let required = if self.is_required() { " *" } else { "" };
        let widget = self.as_widget();
        let errors = if self.has_errors() {
            format!(
                r#"<ul class="errorlist">{}</ul>"#,
                self.errors()
                    .iter()
                    .map(|e| format!("<li>{}</li>", html_escape(e)))
                    .collect::<String>()
            )
        } else {
            String::new()
        };

        format!(
            r#"<div class="field {}"><label for="{}">{}{}</label>{}{}</div>"#,
            if self.has_errors() { "error" } else { "" },
            self.id_for_label(),
            html_escape(label),
            required,
            widget,
            errors
        )
    }
}

/// HTML escape helper
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::CharField;

    #[test]
    fn test_bound_field_basic() {
        let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
        let data = serde_json::json!("John Doe");
        let errors = vec![];

        let bound = BoundField::new("test_form".to_string(), &field, Some(&data), &errors, "");

        assert_eq!(bound.name(), "name");
        assert_eq!(bound.html_name(), "name");
        assert_eq!(bound.id_for_label(), "id_name");
        assert_eq!(bound.value(), Some(&data));
        assert!(!bound.has_errors());
    }

    #[test]
    fn test_bound_field_with_prefix() {
        let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
        let data = serde_json::json!("John Doe");
        let errors = vec![];

        let bound = BoundField::new(
            "test_form".to_string(),
            &field,
            Some(&data),
            &errors,
            "profile",
        );

        assert_eq!(bound.html_name(), "profile-name");
        assert_eq!(bound.id_for_label(), "id_profile-name");
    }

    #[test]
    fn test_bound_field_with_errors() {
        let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
        let data = serde_json::json!("");
        let errors = vec!["This field is required.".to_string()];

        let bound = BoundField::new("test_form".to_string(), &field, Some(&data), &errors, "");

        assert!(bound.has_errors());
        assert_eq!(bound.errors().len(), 1);
    }

    #[test]
    fn test_forms_bound_field_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_as_widget_rendering() {
        let field: Box<dyn FormField> = Box::new(CharField::new("name".to_string()));
        let data = serde_json::json!("Test Value");
        let errors = vec![];

        let bound = BoundField::new("test_form".to_string(), &field, Some(&data), &errors, "");
        let html = bound.as_widget();

        assert!(html.contains("type=\"text\""));
        assert!(html.contains("name=\"name\""));
        assert!(html.contains("value=\"Test Value\""));
    }
}
