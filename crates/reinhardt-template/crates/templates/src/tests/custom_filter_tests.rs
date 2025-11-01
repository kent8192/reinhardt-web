//! Custom filter tests
//!
//! Tests for custom template filters inspired by Django filter tests

use crate::custom_filters::*;
use std::collections::HashMap;
use tera::Value;

#[test]
fn test_upper_filter() {
	// Test uppercase conversion
	let args = HashMap::new();
	assert_eq!(
		upper(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("HELLO".to_string())
	);
	assert_eq!(
		upper(&Value::String("world".to_string()), &args).unwrap(),
		Value::String("WORLD".to_string())
	);
	assert_eq!(
		upper(&Value::String("HeLLo WoRLd".to_string()), &args).unwrap(),
		Value::String("HELLO WORLD".to_string())
	);
}

#[test]
fn test_upper_filter_empty() {
	// Test uppercase with empty string
	let args = HashMap::new();
	assert_eq!(
		upper(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_upper_filter_unicode() {
	// Test uppercase with Unicode
	let args = HashMap::new();
	assert_eq!(
		upper(&Value::String("こんにちは".to_string()), &args).unwrap(),
		Value::String("こんにちは".to_string())
	);
	assert_eq!(
		upper(&Value::String("café".to_string()), &args).unwrap(),
		Value::String("CAFÉ".to_string())
	);
}

#[test]
fn test_lower_filter() {
	// Test lowercase conversion
	let args = HashMap::new();
	assert_eq!(
		lower(&Value::String("HELLO".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		lower(&Value::String("WORLD".to_string()), &args).unwrap(),
		Value::String("world".to_string())
	);
	assert_eq!(
		lower(&Value::String("HeLLo WoRLd".to_string()), &args).unwrap(),
		Value::String("hello world".to_string())
	);
}

#[test]
fn test_lower_filter_empty() {
	// Test lowercase with empty string
	let args = HashMap::new();
	assert_eq!(
		lower(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_lower_filter_unicode() {
	// Test lowercase with Unicode
	let args = HashMap::new();
	assert_eq!(
		lower(&Value::String("CAFÉ".to_string()), &args).unwrap(),
		Value::String("café".to_string())
	);
}

#[test]
fn test_trim_filter() {
	// Test trimming whitespace
	let args = HashMap::new();
	assert_eq!(
		trim(&Value::String("  hello  ".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		trim(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		trim(&Value::String("  hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		trim(&Value::String("hello  ".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
}

#[test]
fn test_trim_filter_tabs_newlines() {
	// Test trimming tabs and newlines
	let args = HashMap::new();
	assert_eq!(
		trim(&Value::String("\t\nhello\t\n".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		trim(&Value::String("\r\n  hello  \r\n".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
}

#[test]
fn test_trim_filter_empty() {
	// Test trimming empty or whitespace-only strings
	let args = HashMap::new();
	assert_eq!(
		trim(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
	assert_eq!(
		trim(&Value::String("   ".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
	assert_eq!(
		trim(&Value::String("\t\n".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_reverse_filter() {
	// Test string reversal
	let args = HashMap::new();
	assert_eq!(
		reverse(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("olleh".to_string())
	);
	assert_eq!(
		reverse(&Value::String("world".to_string()), &args).unwrap(),
		Value::String("dlrow".to_string())
	);
	assert_eq!(
		reverse(&Value::String("12345".to_string()), &args).unwrap(),
		Value::String("54321".to_string())
	);
}

#[test]
fn test_reverse_filter_empty() {
	// Test reversing empty string
	let args = HashMap::new();
	assert_eq!(
		reverse(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_reverse_filter_palindrome() {
	// Test reversing palindromes
	let args = HashMap::new();
	assert_eq!(
		reverse(&Value::String("racecar".to_string()), &args).unwrap(),
		Value::String("racecar".to_string())
	);
	assert_eq!(
		reverse(&Value::String("noon".to_string()), &args).unwrap(),
		Value::String("noon".to_string())
	);
}

#[test]
fn test_truncate_filter() {
	// Test string truncation
	let mut args = HashMap::new();
	args.insert("length".to_string(), Value::Number(5.into()));
	assert_eq!(
		truncate(&Value::String("Hello World".to_string()), &args).unwrap(),
		Value::String("Hello...".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("length".to_string(), Value::Number(11.into()));
	assert_eq!(
		truncate(&Value::String("Hello World".to_string()), &args2).unwrap(),
		Value::String("Hello World".to_string())
	);

	let mut args3 = HashMap::new();
	args3.insert("length".to_string(), Value::Number(20.into()));
	assert_eq!(
		truncate(&Value::String("Hello World".to_string()), &args3).unwrap(),
		Value::String("Hello World".to_string())
	);
}

#[test]
fn test_truncate_filter_exact_length() {
	// Test truncation at exact length
	let mut args = HashMap::new();
	args.insert("length".to_string(), Value::Number(5.into()));
	assert_eq!(
		truncate(&Value::String("Hello".to_string()), &args).unwrap(),
		Value::String("Hello".to_string())
	);
}

#[test]
fn test_truncate_filter_zero() {
	// Test truncation with zero length
	let mut args = HashMap::new();
	args.insert("length".to_string(), Value::Number(0.into()));
	assert_eq!(
		truncate(&Value::String("Hello".to_string()), &args).unwrap(),
		Value::String("...".to_string())
	);
}

#[test]
fn test_join_filter() {
	// Test joining strings
	let items = vec![
		Value::String("a".to_string()),
		Value::String("b".to_string()),
		Value::String("c".to_string()),
	];
	let value = Value::Array(items);

	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(", ".to_string()));
	assert_eq!(
		join(&value, &args).unwrap(),
		Value::String("a, b, c".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("sep".to_string(), Value::String("-".to_string()));
	assert_eq!(
		join(&value, &args2).unwrap(),
		Value::String("a-b-c".to_string())
	);

	let mut args3 = HashMap::new();
	args3.insert("sep".to_string(), Value::String("".to_string()));
	assert_eq!(
		join(&value, &args3).unwrap(),
		Value::String("abc".to_string())
	);
}

#[test]
fn test_join_filter_empty() {
	// Test joining empty list
	let items: Vec<Value> = vec![];
	let value = Value::Array(items);

	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(", ".to_string()));
	assert_eq!(join(&value, &args).unwrap(), Value::String("".to_string()));
}

#[test]
fn test_join_filter_single() {
	// Test joining single item
	let items = vec![Value::String("only".to_string())];
	let value = Value::Array(items);

	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(", ".to_string()));
	assert_eq!(
		join(&value, &args).unwrap(),
		Value::String("only".to_string())
	);
}

#[test]
fn test_default_filter() {
	// Test default value
	let mut args = HashMap::new();
	args.insert("value".to_string(), Value::String("N/A".to_string()));

	assert_eq!(
		default(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
	assert_eq!(
		default(&Value::String("".to_string()), &args).unwrap(),
		Value::String("N/A".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("value".to_string(), Value::String("default".to_string()));
	assert_eq!(
		default(&Value::String("".to_string()), &args2).unwrap(),
		Value::String("default".to_string())
	);
}

#[test]
fn test_default_filter_whitespace() {
	// Test default with whitespace (should not be treated as empty)
	let mut args = HashMap::new();
	args.insert("value".to_string(), Value::String("N/A".to_string()));

	assert_eq!(
		default(&Value::String(" ".to_string()), &args).unwrap(),
		Value::String(" ".to_string())
	);
}

#[test]
fn test_capitalize_filter() {
	// Test capitalization
	let args = HashMap::new();
	assert_eq!(
		capitalize(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("Hello".to_string())
	);
	assert_eq!(
		capitalize(&Value::String("hello world".to_string()), &args).unwrap(),
		Value::String("Hello world".to_string())
	);
	assert_eq!(
		capitalize(&Value::String("HELLO".to_string()), &args).unwrap(),
		Value::String("HELLO".to_string())
	);
}

#[test]
fn test_capitalize_filter_empty() {
	// Test capitalizing empty string
	let args = HashMap::new();
	assert_eq!(
		capitalize(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_title_filter() {
	// Test title case
	let args = HashMap::new();
	assert_eq!(
		title(&Value::String("hello world".to_string()), &args).unwrap(),
		Value::String("Hello World".to_string())
	);
	assert_eq!(
		title(&Value::String("the quick brown fox".to_string()), &args).unwrap(),
		Value::String("The Quick Brown Fox".to_string())
	);
	assert_eq!(
		title(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_title_filter_single_word() {
	// Test title case with single word
	let args = HashMap::new();
	assert_eq!(
		title(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("Hello".to_string())
	);
}

#[test]
fn test_title_filter_already_titled() {
	// Test title case with already titled string
	let args = HashMap::new();
	assert_eq!(
		title(&Value::String("Hello World".to_string()), &args).unwrap(),
		Value::String("Hello World".to_string())
	);
}

#[test]
fn test_length_filter() {
	// Test length calculation
	let args = HashMap::new();
	assert_eq!(
		length(&Value::String("hello".to_string()), &args).unwrap(),
		Value::Number(5.into())
	);
	assert_eq!(
		length(&Value::String("".to_string()), &args).unwrap(),
		Value::Number(0.into())
	);
	assert_eq!(
		length(&Value::String("hello world".to_string()), &args).unwrap(),
		Value::Number(11.into())
	);
}

#[test]
fn test_length_filter_unicode() {
	// Test length with Unicode (counts bytes, not characters)
	let args = HashMap::new();
	assert_eq!(
		length(&Value::String("café".to_string()), &args).unwrap(),
		Value::Number(5.into())
	); // é is 2 bytes
}

#[test]
fn test_ljust_filter() {
	// Test left justification
	let mut args = HashMap::new();
	args.insert("width".to_string(), Value::Number(5.into()));
	args.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		ljust(&Value::String("42".to_string()), &args).unwrap(),
		Value::String("42000".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("width".to_string(), Value::Number(8.into()));
	args2.insert("fill".to_string(), Value::String("-".to_string()));
	assert_eq!(
		ljust(&Value::String("test".to_string()), &args2).unwrap(),
		Value::String("test----".to_string())
	);
}

#[test]
fn test_ljust_filter_no_padding() {
	// Test ljust when string is already long enough
	let mut args = HashMap::new();
	args.insert("width".to_string(), Value::Number(3.into()));
	args.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		ljust(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("width".to_string(), Value::Number(5.into()));
	args2.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		ljust(&Value::String("hello".to_string()), &args2).unwrap(),
		Value::String("hello".to_string())
	);
}

#[test]
fn test_rjust_filter() {
	// Test right justification
	let mut args = HashMap::new();
	args.insert("width".to_string(), Value::Number(5.into()));
	args.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		rjust(&Value::String("42".to_string()), &args).unwrap(),
		Value::String("00042".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("width".to_string(), Value::Number(8.into()));
	args2.insert("fill".to_string(), Value::String("-".to_string()));
	assert_eq!(
		rjust(&Value::String("test".to_string()), &args2).unwrap(),
		Value::String("----test".to_string())
	);
}

#[test]
fn test_rjust_filter_no_padding() {
	// Test rjust when string is already long enough
	let mut args = HashMap::new();
	args.insert("width".to_string(), Value::Number(3.into()));
	args.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		rjust(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("width".to_string(), Value::Number(5.into()));
	args2.insert("fill".to_string(), Value::String("0".to_string()));
	assert_eq!(
		rjust(&Value::String("hello".to_string()), &args2).unwrap(),
		Value::String("hello".to_string())
	);
}

#[test]
fn test_replace_filter() {
	// Test string replacement
	let mut args = HashMap::new();
	args.insert("from".to_string(), Value::String("world".to_string()));
	args.insert("to".to_string(), Value::String("rust".to_string()));
	assert_eq!(
		replace(&Value::String("hello world".to_string()), &args).unwrap(),
		Value::String("hello rust".to_string())
	);

	let mut args2 = HashMap::new();
	args2.insert("from".to_string(), Value::String("abc".to_string()));
	args2.insert("to".to_string(), Value::String("xyz".to_string()));
	assert_eq!(
		replace(&Value::String("abc abc abc".to_string()), &args2).unwrap(),
		Value::String("xyz xyz xyz".to_string())
	);
}

#[test]
fn test_replace_filter_no_match() {
	// Test replacement when pattern not found
	let mut args = HashMap::new();
	args.insert("from".to_string(), Value::String("world".to_string()));
	args.insert("to".to_string(), Value::String("rust".to_string()));
	assert_eq!(
		replace(&Value::String("hello".to_string()), &args).unwrap(),
		Value::String("hello".to_string())
	);
}

#[test]
fn test_replace_filter_empty() {
	// Test replacement with empty strings
	let mut args = HashMap::new();
	args.insert("from".to_string(), Value::String("a".to_string()));
	args.insert("to".to_string(), Value::String("b".to_string()));
	assert_eq!(
		replace(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);

	// Note: Rust's str::replace with empty pattern inserts between every character
	// This is expected behavior
	let mut args2 = HashMap::new();
	args2.insert("from".to_string(), Value::String("".to_string()));
	args2.insert("to".to_string(), Value::String("x".to_string()));
	assert_eq!(
		replace(&Value::String("hello".to_string()), &args2).unwrap(),
		Value::String("xhxexlxlxox".to_string())
	);
}

#[test]
fn test_split_filter() {
	// Test string splitting
	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(",".to_string()));

	let result = split(&Value::String("a,b,c".to_string()), &args).unwrap();
	let expected = Value::Array(vec![
		Value::String("a".to_string()),
		Value::String("b".to_string()),
		Value::String("c".to_string()),
	]);
	assert_eq!(result, expected);

	let mut args2 = HashMap::new();
	args2.insert("sep".to_string(), Value::String("-".to_string()));

	let result2 = split(&Value::String("one-two-three".to_string()), &args2).unwrap();
	let expected2 = Value::Array(vec![
		Value::String("one".to_string()),
		Value::String("two".to_string()),
		Value::String("three".to_string()),
	]);
	assert_eq!(result2, expected2);
}

#[test]
fn test_split_filter_no_separator() {
	// Test splitting when separator not found
	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(",".to_string()));

	let result = split(&Value::String("hello".to_string()), &args).unwrap();
	let expected = Value::Array(vec![Value::String("hello".to_string())]);
	assert_eq!(result, expected);
}

#[test]
fn test_split_filter_multiple_separators() {
	// Test splitting with consecutive separators
	let mut args = HashMap::new();
	args.insert("sep".to_string(), Value::String(",".to_string()));

	let result = split(&Value::String("a,,b".to_string()), &args).unwrap();
	let expected = Value::Array(vec![
		Value::String("a".to_string()),
		Value::String("".to_string()),
		Value::String("b".to_string()),
	]);
	assert_eq!(result, expected);
}

#[test]
fn test_striptags_filter() {
	// Test HTML tag stripping
	let args = HashMap::new();
	assert_eq!(
		striptags(&Value::String("<p>Hello</p>".to_string()), &args).unwrap(),
		Value::String("Hello".to_string())
	);
	assert_eq!(
		striptags(
			&Value::String("<div><p>Nested</p></div>".to_string()),
			&args
		)
		.unwrap(),
		Value::String("Nested".to_string())
	);
	assert_eq!(
		striptags(&Value::String("<a href='#'>Link</a>".to_string()), &args).unwrap(),
		Value::String("Link".to_string())
	);
}

#[test]
fn test_striptags_filter_no_tags() {
	// Test striptags with no HTML tags
	let args = HashMap::new();
	assert_eq!(
		striptags(&Value::String("No tags here".to_string()), &args).unwrap(),
		Value::String("No tags here".to_string())
	);
}

#[test]
fn test_striptags_filter_empty() {
	// Test striptags with empty string
	let args = HashMap::new();
	assert_eq!(
		striptags(&Value::String("".to_string()), &args).unwrap(),
		Value::String("".to_string())
	);
}

#[test]
fn test_striptags_filter_malformed() {
	// Test striptags with malformed HTML
	let args = HashMap::new();
	assert_eq!(
		striptags(&Value::String("<p>Unclosed".to_string()), &args).unwrap(),
		Value::String("Unclosed".to_string())
	);
	assert_eq!(
		striptags(&Value::String("No opening tag</p>".to_string()), &args).unwrap(),
		Value::String("No opening tag".to_string())
	);
}

#[test]
fn test_filter_chaining_concept() {
	// Test that filters can be chained (conceptual test)
	// In actual templates: {{ "  HELLO  "|trim|lower }}
	let args = HashMap::new();

	let result = trim(&Value::String("  HELLO  ".to_string()), &args).unwrap();
	let result = lower(&result, &args).unwrap();
	assert_eq!(result, Value::String("hello".to_string()));
}

#[test]
fn test_filter_combination_upper_reverse() {
	// Test combining upper and reverse filters
	let args = HashMap::new();

	let result = upper(&Value::String("hello".to_string()), &args).unwrap();
	let result = reverse(&result, &args).unwrap();
	assert_eq!(result, Value::String("OLLEH".to_string()));
}

#[test]
fn test_filter_combination_truncate_default() {
	// Test combining truncate and default filters
	// Empty string truncated becomes "...", which is not empty
	let mut args = HashMap::new();
	args.insert("length".to_string(), Value::Number(5.into()));

	let result = truncate(&Value::String("".to_string()), &args).unwrap();
	assert_eq!(result, Value::String("".to_string())); // Empty string stays empty when <= length

	// Use a non-empty result
	let result = truncate(&Value::String("Hello World".to_string()), &args).unwrap();

	let mut args2 = HashMap::new();
	args2.insert("value".to_string(), Value::String("N/A".to_string()));

	let result = default(&result, &args2).unwrap();
	assert_eq!(result, Value::String("Hello...".to_string())); // Not empty, so original value used
}
