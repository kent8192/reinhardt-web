//! Template context tests
//!
//! Tests for template context management inspired by Django's test_context.py

use askama::Template;

#[derive(Template)]
#[template(source = "Value: {{ value }}", ext = "txt")]
struct SimpleTemplate {
    value: String,
}

#[derive(Template)]
#[template(source = "{{ key1 }} - {{ key2 }}", ext = "txt")]
struct MultiValueTemplate {
    key1: String,
    key2: String,
}

#[derive(Template)]
#[template(
    source = "{% for item in items %}{{ item }}{% if !loop.last %}, {% endif %}{% endfor %}",
    ext = "txt"
)]
struct LoopTemplate {
    items: Vec<String>,
}

#[derive(Template)]
#[template(
    source = "{% if show %}{{ content }}{% else %}Hidden{% endif %}",
    ext = "txt"
)]
struct ConditionalTemplate {
    show: bool,
    content: String,
}

#[test]
fn test_simple_context() {
    // Test basic context rendering (similar to Django's test_context)
    let tmpl = SimpleTemplate {
        value: "Test".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Value: Test");
}

#[test]
fn test_multi_value_context() {
    // Test context with multiple values
    let tmpl = MultiValueTemplate {
        key1: "Hello".to_string(),
        key2: "World".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello - World");
}

#[test]
fn test_context_with_empty_string() {
    // Test context with empty string value
    let tmpl = SimpleTemplate {
        value: String::new(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Value: ");
}

#[test]
fn test_context_with_special_characters() {
    // Test context with special characters
    let tmpl = SimpleTemplate {
        value: "Special: <>&\"'".to_string(),
    };

    let result = tmpl.render();
    assert!(result.is_ok());
    // Askama automatically escapes HTML by default
    assert!(result.unwrap().contains("Special"));
}

#[test]
fn test_loop_context() {
    // Test context with loop (similar to Django's context iteration)
    let tmpl = LoopTemplate {
        items: vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ],
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "first, second, third");
}

#[test]
fn test_loop_context_empty() {
    // Test loop with empty list
    let tmpl = LoopTemplate { items: vec![] };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_loop_context_single_item() {
    // Test loop with single item
    let tmpl = LoopTemplate {
        items: vec!["only".to_string()],
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "only");
}

#[test]
fn test_conditional_context_true() {
    // Test conditional rendering when condition is true
    let tmpl = ConditionalTemplate {
        show: true,
        content: "Visible".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Visible");
}

#[test]
fn test_conditional_context_false() {
    // Test conditional rendering when condition is false
    let tmpl = ConditionalTemplate {
        show: false,
        content: "Should not appear".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hidden");
}

#[test]
fn test_context_comparable() {
    // Test that we can compare context values (similar to Django's test_context_comparable)
    let tmpl1 = SimpleTemplate {
        value: "Same".to_string(),
    };
    let tmpl2 = SimpleTemplate {
        value: "Same".to_string(),
    };

    assert_eq!(tmpl1.render().unwrap(), tmpl2.render().unwrap());
}

#[test]
fn test_context_with_numbers() {
    #[derive(Template)]
    #[template(source = "Number: {{ num }}", ext = "txt")]
    struct NumberTemplate {
        num: i32,
    }

    let tmpl = NumberTemplate { num: 42 };
    let result = tmpl.render().unwrap();
    assert_eq!(result, "Number: 42");
}

#[test]
fn test_context_with_boolean() {
    #[derive(Template)]
    #[template(source = "{% if flag %}yes{% else %}no{% endif %}", ext = "txt")]
    struct BoolTemplate {
        flag: bool,
    }

    let tmpl_true = BoolTemplate { flag: true };
    assert_eq!(tmpl_true.render().unwrap(), "yes");

    let tmpl_false = BoolTemplate { flag: false };
    assert_eq!(tmpl_false.render().unwrap(), "no");
}

#[test]
fn test_context_with_nested_struct() {
    #[derive(Template)]
    #[template(source = "{{ user.name }} ({{ user.age }})", ext = "txt")]
    struct UserTemplate {
        user: User,
    }

    struct User {
        name: String,
        age: u32,
    }

    let tmpl = UserTemplate {
        user: User {
            name: "Alice".to_string(),
            age: 30,
        },
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Alice (30)");
}

#[test]
fn test_context_with_option() {
    #[derive(Template)]
    #[template(
        source = "{% if let Some(val) = opt %}{{ val }}{% else %}None{% endif %}",
        ext = "txt"
    )]
    struct OptionTemplate {
        opt: Option<String>,
    }

    let tmpl_some = OptionTemplate {
        opt: Some("Value".to_string()),
    };
    assert_eq!(tmpl_some.render().unwrap(), "Value");

    let tmpl_none = OptionTemplate { opt: None };
    assert_eq!(tmpl_none.render().unwrap(), "None");
}
