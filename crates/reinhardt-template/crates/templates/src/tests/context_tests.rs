//! Template context tests
//!
//! Tests for template context management inspired by Django's test_context.py

use serde::Serialize;
use tera::{Context, Tera};

#[test]
fn test_simple_context() {
	// Test basic context rendering (similar to Django's test_context)
	let mut context = Context::new();
	context.insert("value", "Test");

	let result = Tera::one_off("Value: {{ value }}", &context, false).unwrap();
	assert_eq!(result, "Value: Test");
}

#[test]
fn test_multi_value_context() {
	// Test context with multiple values
	let mut context = Context::new();
	context.insert("key1", "Hello");
	context.insert("key2", "World");

	let result = Tera::one_off("{{ key1 }} - {{ key2 }}", &context, false).unwrap();
	assert_eq!(result, "Hello - World");
}

#[test]
fn test_context_with_empty_string() {
	// Test context with empty string value
	let mut context = Context::new();
	context.insert("value", "");

	let result = Tera::one_off("Value: {{ value }}", &context, false).unwrap();
	assert_eq!(result, "Value: ");
}

#[test]
fn test_context_with_special_characters() {
	// Test context with special characters
	let mut context = Context::new();
	context.insert("value", "Special: <>&\"'");

	let result = Tera::one_off("Value: {{ value }}", &context, false);
	assert!(result.is_ok());
	// Tera automatically escapes HTML by default
	assert!(result.unwrap().contains("Special"));
}

#[test]
fn test_loop_context() {
	// Test context with loop (similar to Django's context iteration)
	let mut context = Context::new();
	context.insert("items", &vec!["first", "second", "third"]);

	let template =
		"{% for item in items %}{{ item }}{% if not loop.last %}, {% endif %}{% endfor %}";
	let result = Tera::one_off(template, &context, false).unwrap();
	assert_eq!(result, "first, second, third");
}

#[test]
fn test_loop_context_empty() {
	// Test loop with empty list
	let mut context = Context::new();
	context.insert("items", &Vec::<String>::new());

	let template =
		"{% for item in items %}{{ item }}{% if not loop.last %}, {% endif %}{% endfor %}";
	let result = Tera::one_off(template, &context, false).unwrap();
	assert_eq!(result, "");
}

#[test]
fn test_loop_context_single_item() {
	// Test loop with single item
	let mut context = Context::new();
	context.insert("items", &vec!["only"]);

	let template =
		"{% for item in items %}{{ item }}{% if not loop.last %}, {% endif %}{% endfor %}";
	let result = Tera::one_off(template, &context, false).unwrap();
	assert_eq!(result, "only");
}

#[test]
fn test_conditional_context_true() {
	// Test conditional rendering when condition is true
	let mut context = Context::new();
	context.insert("show", &true);
	context.insert("content", "Visible");

	let template = "{% if show %}{{ content }}{% else %}Hidden{% endif %}";
	let result = Tera::one_off(template, &context, false).unwrap();
	assert_eq!(result, "Visible");
}

#[test]
fn test_conditional_context_false() {
	// Test conditional rendering when condition is false
	let mut context = Context::new();
	context.insert("show", &false);
	context.insert("content", "Should not appear");

	let template = "{% if show %}{{ content }}{% else %}Hidden{% endif %}";
	let result = Tera::one_off(template, &context, false).unwrap();
	assert_eq!(result, "Hidden");
}

#[test]
fn test_context_comparable() {
	// Test that we can compare context values (similar to Django's test_context_comparable)
	let mut context1 = Context::new();
	context1.insert("value", "Same");

	let mut context2 = Context::new();
	context2.insert("value", "Same");

	let result1 = Tera::one_off("Value: {{ value }}", &context1, false).unwrap();
	let result2 = Tera::one_off("Value: {{ value }}", &context2, false).unwrap();

	assert_eq!(result1, result2);
}

#[test]
fn test_context_with_numbers() {
	let mut context = Context::new();
	context.insert("num", &42);

	let result = Tera::one_off("Number: {{ num }}", &context, false).unwrap();
	assert_eq!(result, "Number: 42");
}

#[test]
fn test_context_with_boolean() {
	let mut context_true = Context::new();
	context_true.insert("flag", &true);

	let template = "{% if flag %}yes{% else %}no{% endif %}";
	assert_eq!(
		Tera::one_off(template, &context_true, false).unwrap(),
		"yes"
	);

	let mut context_false = Context::new();
	context_false.insert("flag", &false);

	assert_eq!(
		Tera::one_off(template, &context_false, false).unwrap(),
		"no"
	);
}

#[test]
fn test_context_with_nested_struct() {
	#[derive(Serialize)]
	struct User {
		name: String,
		age: u32,
	}

	let mut context = Context::new();
	context.insert(
		"user",
		&User {
			name: "Alice".to_string(),
			age: 30,
		},
	);

	let result = Tera::one_off("{{ user.name }} ({{ user.age }})", &context, false).unwrap();
	assert_eq!(result, "Alice (30)");
}

#[test]
fn test_context_with_option() {
	let mut context_some = Context::new();
	context_some.insert("opt", &Some("Value".to_string()));

	let template = "{% if opt %}{{ opt }}{% else %}None{% endif %}";
	assert_eq!(
		Tera::one_off(template, &context_some, false).unwrap(),
		"Value"
	);

	let mut context_none = Context::new();
	context_none.insert("opt", &Option::<String>::None);

	assert_eq!(
		Tera::one_off(template, &context_none, false).unwrap(),
		"None"
	);
}
