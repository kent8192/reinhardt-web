//! Custom filter tests
//!
//! Tests for custom template filters inspired by Django filter tests

use crate::custom_filters::*;

#[test]
fn test_upper_filter() {
    // Test uppercase conversion
    assert_eq!(upper("hello").unwrap(), "HELLO");
    assert_eq!(upper("world").unwrap(), "WORLD");
    assert_eq!(upper("HeLLo WoRLd").unwrap(), "HELLO WORLD");
}

#[test]
fn test_upper_filter_empty() {
    // Test uppercase with empty string
    assert_eq!(upper("").unwrap(), "");
}

#[test]
fn test_upper_filter_unicode() {
    // Test uppercase with Unicode
    assert_eq!(upper("こんにちは").unwrap(), "こんにちは");
    assert_eq!(upper("café").unwrap(), "CAFÉ");
}

#[test]
fn test_lower_filter() {
    // Test lowercase conversion
    assert_eq!(lower("HELLO").unwrap(), "hello");
    assert_eq!(lower("WORLD").unwrap(), "world");
    assert_eq!(lower("HeLLo WoRLd").unwrap(), "hello world");
}

#[test]
fn test_lower_filter_empty() {
    // Test lowercase with empty string
    assert_eq!(lower("").unwrap(), "");
}

#[test]
fn test_lower_filter_unicode() {
    // Test lowercase with Unicode
    assert_eq!(lower("CAFÉ").unwrap(), "café");
}

#[test]
fn test_trim_filter() {
    // Test trimming whitespace
    assert_eq!(trim("  hello  ").unwrap(), "hello");
    assert_eq!(trim("hello").unwrap(), "hello");
    assert_eq!(trim("  hello").unwrap(), "hello");
    assert_eq!(trim("hello  ").unwrap(), "hello");
}

#[test]
fn test_trim_filter_tabs_newlines() {
    // Test trimming tabs and newlines
    assert_eq!(trim("\t\nhello\t\n").unwrap(), "hello");
    assert_eq!(trim("\r\n  hello  \r\n").unwrap(), "hello");
}

#[test]
fn test_trim_filter_empty() {
    // Test trimming empty or whitespace-only strings
    assert_eq!(trim("").unwrap(), "");
    assert_eq!(trim("   ").unwrap(), "");
    assert_eq!(trim("\t\n").unwrap(), "");
}

#[test]
fn test_reverse_filter() {
    // Test string reversal
    assert_eq!(reverse("hello").unwrap(), "olleh");
    assert_eq!(reverse("world").unwrap(), "dlrow");
    assert_eq!(reverse("12345").unwrap(), "54321");
}

#[test]
fn test_reverse_filter_empty() {
    // Test reversing empty string
    assert_eq!(reverse("").unwrap(), "");
}

#[test]
fn test_reverse_filter_palindrome() {
    // Test reversing palindromes
    assert_eq!(reverse("racecar").unwrap(), "racecar");
    assert_eq!(reverse("noon").unwrap(), "noon");
}

#[test]
fn test_truncate_filter() {
    // Test string truncation
    assert_eq!(truncate("Hello World", 5).unwrap(), "Hello...");
    assert_eq!(truncate("Hello World", 11).unwrap(), "Hello World");
    assert_eq!(truncate("Hello World", 20).unwrap(), "Hello World");
}

#[test]
fn test_truncate_filter_exact_length() {
    // Test truncation at exact length
    assert_eq!(truncate("Hello", 5).unwrap(), "Hello");
}

#[test]
fn test_truncate_filter_zero() {
    // Test truncation with zero length
    assert_eq!(truncate("Hello", 0).unwrap(), "...");
}

#[test]
fn test_join_filter() {
    // Test joining strings
    let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(join(&items, ", ").unwrap(), "a, b, c");
    assert_eq!(join(&items, "-").unwrap(), "a-b-c");
    assert_eq!(join(&items, "").unwrap(), "abc");
}

#[test]
fn test_join_filter_empty() {
    // Test joining empty list
    let items: Vec<String> = vec![];
    assert_eq!(join(&items, ", ").unwrap(), "");
}

#[test]
fn test_join_filter_single() {
    // Test joining single item
    let items = vec!["only".to_string()];
    assert_eq!(join(&items, ", ").unwrap(), "only");
}

#[test]
fn test_default_filter() {
    // Test default value
    assert_eq!(default("hello", "N/A").unwrap(), "hello");
    assert_eq!(default("", "N/A").unwrap(), "N/A");
    assert_eq!(default("", "default").unwrap(), "default");
}

#[test]
fn test_default_filter_whitespace() {
    // Test default with whitespace (should not be treated as empty)
    assert_eq!(default(" ", "N/A").unwrap(), " ");
}

#[test]
fn test_capitalize_filter() {
    // Test capitalization
    assert_eq!(capitalize("hello").unwrap(), "Hello");
    assert_eq!(capitalize("hello world").unwrap(), "Hello world");
    assert_eq!(capitalize("HELLO").unwrap(), "HELLO");
}

#[test]
fn test_capitalize_filter_empty() {
    // Test capitalizing empty string
    assert_eq!(capitalize("").unwrap(), "");
}

#[test]
fn test_title_filter() {
    // Test title case
    assert_eq!(title("hello world").unwrap(), "Hello World");
    assert_eq!(title("the quick brown fox").unwrap(), "The Quick Brown Fox");
    assert_eq!(title("").unwrap(), "");
}

#[test]
fn test_title_filter_single_word() {
    // Test title case with single word
    assert_eq!(title("hello").unwrap(), "Hello");
}

#[test]
fn test_title_filter_already_titled() {
    // Test title case with already titled string
    assert_eq!(title("Hello World").unwrap(), "Hello World");
}

#[test]
fn test_length_filter() {
    // Test length calculation
    assert_eq!(length("hello").unwrap(), 5);
    assert_eq!(length("").unwrap(), 0);
    assert_eq!(length("hello world").unwrap(), 11);
}

#[test]
fn test_length_filter_unicode() {
    // Test length with Unicode (counts bytes, not characters)
    assert_eq!(length("café").unwrap(), 5); // é is 2 bytes
}

#[test]
fn test_ljust_filter() {
    // Test left justification
    assert_eq!(ljust("42", 5, "0").unwrap(), "42000");
    assert_eq!(ljust("test", 8, "-").unwrap(), "test----");
}

#[test]
fn test_ljust_filter_no_padding() {
    // Test ljust when string is already long enough
    assert_eq!(ljust("hello", 3, "0").unwrap(), "hello");
    assert_eq!(ljust("hello", 5, "0").unwrap(), "hello");
}

#[test]
fn test_rjust_filter() {
    // Test right justification
    assert_eq!(rjust("42", 5, "0").unwrap(), "00042");
    assert_eq!(rjust("test", 8, "-").unwrap(), "----test");
}

#[test]
fn test_rjust_filter_no_padding() {
    // Test rjust when string is already long enough
    assert_eq!(rjust("hello", 3, "0").unwrap(), "hello");
    assert_eq!(rjust("hello", 5, "0").unwrap(), "hello");
}

#[test]
fn test_replace_filter() {
    // Test string replacement
    assert_eq!(
        replace("hello world", "world", "rust").unwrap(),
        "hello rust"
    );
    assert_eq!(replace("abc abc abc", "abc", "xyz").unwrap(), "xyz xyz xyz");
}

#[test]
fn test_replace_filter_no_match() {
    // Test replacement when pattern not found
    assert_eq!(replace("hello", "world", "rust").unwrap(), "hello");
}

#[test]
fn test_replace_filter_empty() {
    // Test replacement with empty strings
    assert_eq!(replace("", "a", "b").unwrap(), "");
    // Note: Rust's str::replace with empty pattern inserts between every character
    // This is expected behavior
    assert_eq!(replace("hello", "", "x").unwrap(), "xhxexlxlxox");
}

#[test]
fn test_split_filter() {
    // Test string splitting
    assert_eq!(
        split("a,b,c", ",").unwrap(),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(
        split("one-two-three", "-").unwrap(),
        vec!["one".to_string(), "two".to_string(), "three".to_string()]
    );
}

#[test]
fn test_split_filter_no_separator() {
    // Test splitting when separator not found
    assert_eq!(split("hello", ",").unwrap(), vec!["hello".to_string()]);
}

#[test]
fn test_split_filter_multiple_separators() {
    // Test splitting with consecutive separators
    assert_eq!(
        split("a,,b", ",").unwrap(),
        vec!["a".to_string(), "".to_string(), "b".to_string()]
    );
}

#[test]
fn test_striptags_filter() {
    // Test HTML tag stripping
    assert_eq!(striptags("<p>Hello</p>").unwrap(), "Hello");
    assert_eq!(striptags("<div><p>Nested</p></div>").unwrap(), "Nested");
    assert_eq!(striptags("<a href='#'>Link</a>").unwrap(), "Link");
}

#[test]
fn test_striptags_filter_no_tags() {
    // Test striptags with no HTML tags
    assert_eq!(striptags("No tags here").unwrap(), "No tags here");
}

#[test]
fn test_striptags_filter_empty() {
    // Test striptags with empty string
    assert_eq!(striptags("").unwrap(), "");
}

#[test]
fn test_striptags_filter_malformed() {
    // Test striptags with malformed HTML
    assert_eq!(striptags("<p>Unclosed").unwrap(), "Unclosed");
    assert_eq!(striptags("No opening tag</p>").unwrap(), "No opening tag");
}

#[test]
fn test_filter_chaining_concept() {
    // Test that filters can be chained (conceptual test)
    // In actual templates: {{ "  HELLO  "|trim|lower }}
    let result = trim("  HELLO  ").unwrap();
    let result = lower(&result).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_filter_combination_upper_reverse() {
    // Test combining upper and reverse filters
    let result = upper("hello").unwrap();
    let result = reverse(&result).unwrap();
    assert_eq!(result, "OLLEH");
}

#[test]
fn test_filter_combination_truncate_default() {
    // Test combining truncate and default filters
    // Empty string truncated becomes "...", which is not empty
    let result = truncate("", 5).unwrap();
    assert_eq!(result, ""); // Empty string stays empty when <= length

    // Use a non-empty result
    let result = truncate("Hello World", 5).unwrap();
    let result = default(&result, "N/A").unwrap();
    assert_eq!(result, "Hello..."); // Not empty, so original value used
}
