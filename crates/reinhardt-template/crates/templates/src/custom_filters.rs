//! Custom template filters for Askama
//!
//! Provides Django-compatible template filters for use in Askama templates.

use askama::Result as AskamaResult;

/// Convert string to uppercase
///
/// # Example
/// ```askama
/// {{ "hello"|upper }}
/// ```
/// Output: `HELLO`
pub fn upper(s: &str) -> AskamaResult<String> {
    Ok(s.to_uppercase())
}

/// Convert string to lowercase
///
/// # Example
/// ```askama
/// {{ "HELLO"|lower }}
/// ```
/// Output: `hello`
pub fn lower(s: &str) -> AskamaResult<String> {
    Ok(s.to_lowercase())
}

/// Trim whitespace from both ends of a string
///
/// # Example
/// ```askama
/// {{ "  hello  "|trim }}
/// ```
/// Output: `hello`
pub fn trim(s: &str) -> AskamaResult<String> {
    Ok(s.trim().to_string())
}

/// Reverse a string
///
/// # Example
/// ```askama
/// {{ "hello"|reverse }}
/// ```
/// Output: `olleh`
pub fn reverse(s: &str) -> AskamaResult<String> {
    Ok(s.chars().rev().collect())
}

/// Truncate a string to a specified length
///
/// If the string is longer than the specified length, it will be truncated
/// and "..." will be appended. Uses character count, not byte count.
///
/// # Example
/// ```askama
/// {{ "Hello World"|truncate(5) }}
/// ```
/// Output: `Hello...`
pub fn truncate(s: &str, length: usize) -> AskamaResult<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= length {
        Ok(s.to_string())
    } else {
        let truncated: String = chars[..length].iter().collect();
        Ok(format!("{}...", truncated))
    }
}

/// Join a list of strings with a separator
///
/// # Example
/// ```askama
/// {{ items|join(", ") }}
/// ```
pub fn join(items: &[String], separator: &str) -> AskamaResult<String> {
    Ok(items.join(separator))
}

/// Provide a default value if the input is empty
///
/// # Example
/// ```askama
/// {{ value|default("N/A") }}
/// ```
pub fn default(s: &str, default_value: &str) -> AskamaResult<String> {
    if s.is_empty() {
        Ok(default_value.to_string())
    } else {
        Ok(s.to_string())
    }
}

/// Capitalize the first character of a string
///
/// # Example
/// ```askama
/// {{ "hello world"|capitalize }}
/// ```
/// Output: `Hello world`
pub fn capitalize(s: &str) -> AskamaResult<String> {
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(String::new()),
        Some(first) => Ok(first.to_uppercase().collect::<String>() + chars.as_str()),
    }
}

/// Convert a string to title case
///
/// # Example
/// ```askama
/// {{ "hello world"|title }}
/// ```
/// Output: `Hello World`
pub fn title(s: &str) -> AskamaResult<String> {
    Ok(s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" "))
}

/// Get the length of a string
///
/// # Example
/// ```askama
/// {{ "hello"|length }}
/// ```
/// Output: `5`
pub fn length(s: &str) -> AskamaResult<usize> {
    Ok(s.len())
}

/// Left-pad a string with a character to a specified width
///
/// # Example
/// ```askama
/// {{ "42"|ljust(5, "0") }}
/// ```
/// Output: `42000`
pub fn ljust(s: &str, width: usize, fill_char: &str) -> AskamaResult<String> {
    let fill = fill_char.chars().next().unwrap_or(' ');
    if s.len() >= width {
        Ok(s.to_string())
    } else {
        let padding = width - s.len();
        Ok(format!("{}{}", s, fill.to_string().repeat(padding)))
    }
}

/// Right-pad a string with a character to a specified width
///
/// # Example
/// ```askama
/// {{ "42"|rjust(5, "0") }}
/// ```
/// Output: `00042`
pub fn rjust(s: &str, width: usize, fill_char: &str) -> AskamaResult<String> {
    let fill = fill_char.chars().next().unwrap_or(' ');
    if s.len() >= width {
        Ok(s.to_string())
    } else {
        let padding = width - s.len();
        Ok(format!("{}{}", fill.to_string().repeat(padding), s))
    }
}

/// Replace all occurrences of a substring
///
/// # Example
/// ```askama
/// {{ "hello world"|replace("world", "rust") }}
/// ```
/// Output: `hello rust`
pub fn replace(s: &str, from: &str, to: &str) -> AskamaResult<String> {
    Ok(s.replace(from, to))
}

/// Split a string by a separator
///
/// # Example
/// ```askama
/// {{ "a,b,c"|split(",") }}
/// ```
pub fn split(s: &str, separator: &str) -> AskamaResult<Vec<String>> {
    Ok(s.split(separator).map(|s| s.to_string()).collect())
}

/// Strip HTML tags from a string
///
/// # Example
/// ```askama
/// {{ "<p>Hello</p>"|striptags }}
/// ```
/// Output: `Hello`
pub fn striptags(s: &str) -> AskamaResult<String> {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper() {
        assert_eq!(upper("hello").unwrap(), "HELLO");
        assert_eq!(upper("").unwrap(), "");
        assert_eq!(upper("HeLLo").unwrap(), "HELLO");
    }

    #[test]
    fn test_lower() {
        assert_eq!(lower("HELLO").unwrap(), "hello");
        assert_eq!(lower("").unwrap(), "");
        assert_eq!(lower("HeLLo").unwrap(), "hello");
    }

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  ").unwrap(), "hello");
        assert_eq!(trim("hello").unwrap(), "hello");
        assert_eq!(trim("  ").unwrap(), "");
    }

    #[test]
    fn test_reverse() {
        assert_eq!(reverse("hello").unwrap(), "olleh");
        assert_eq!(reverse("").unwrap(), "");
        assert_eq!(reverse("abc").unwrap(), "cba");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5).unwrap(), "Hello...");
        assert_eq!(truncate("Hi", 5).unwrap(), "Hi");
        assert_eq!(truncate("", 5).unwrap(), "");
    }

    #[test]
    fn test_join() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(join(&items, ", ").unwrap(), "a, b, c");
        assert_eq!(join(&[], ", ").unwrap(), "");
    }

    #[test]
    fn test_default() {
        assert_eq!(default("hello", "N/A").unwrap(), "hello");
        assert_eq!(default("", "N/A").unwrap(), "N/A");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello").unwrap(), "Hello");
        assert_eq!(capitalize("HELLO").unwrap(), "HELLO");
        assert_eq!(capitalize("").unwrap(), "");
    }

    #[test]
    fn test_title() {
        assert_eq!(title("hello world").unwrap(), "Hello World");
        assert_eq!(title("").unwrap(), "");
        assert_eq!(title("a b c").unwrap(), "A B C");
    }

    #[test]
    fn test_length() {
        assert_eq!(length("hello").unwrap(), 5);
        assert_eq!(length("").unwrap(), 0);
    }

    #[test]
    fn test_ljust() {
        assert_eq!(ljust("42", 5, "0").unwrap(), "42000");
        assert_eq!(ljust("hello", 3, "0").unwrap(), "hello");
    }

    #[test]
    fn test_rjust() {
        assert_eq!(rjust("42", 5, "0").unwrap(), "00042");
        assert_eq!(rjust("hello", 3, "0").unwrap(), "hello");
    }

    #[test]
    fn test_replace() {
        assert_eq!(
            replace("hello world", "world", "rust").unwrap(),
            "hello rust"
        );
        assert_eq!(replace("", "a", "b").unwrap(), "");
    }

    #[test]
    fn test_split() {
        assert_eq!(split("a,b,c", ",").unwrap(), vec!["a", "b", "c"]);
        assert_eq!(split("hello", ",").unwrap(), vec!["hello"]);
    }

    #[test]
    fn test_striptags() {
        assert_eq!(striptags("<p>Hello</p>").unwrap(), "Hello");
        assert_eq!(striptags("No tags").unwrap(), "No tags");
        assert_eq!(striptags("<div><p>Nested</p></div>").unwrap(), "Nested");
    }
}
