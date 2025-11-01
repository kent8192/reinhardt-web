//! Custom template filters for Tera
//!
//! Provides Django-compatible template filters for use in Tera templates.

use std::collections::HashMap;
use tera::{Result as TeraResult, Value};

/// Convert string to uppercase
///
/// # Example
/// ```tera
/// {{ "hello"|upper }}
/// ```
/// Output: `HELLO`
pub fn upper(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("upper filter requires a string")
    })?;
    Ok(Value::String(s.to_uppercase()))
}

/// Convert string to lowercase
///
/// # Example
/// ```tera
/// {{ "HELLO"|lower }}
/// ```
/// Output: `hello`
pub fn lower(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("lower filter requires a string")
    })?;
    Ok(Value::String(s.to_lowercase()))
}

/// Trim whitespace from both ends of a string
///
/// # Example
/// ```tera
/// {{ "  hello  "|trim }}
/// ```
/// Output: `hello`
pub fn trim(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("trim filter requires a string")
    })?;
    Ok(Value::String(s.trim().to_string()))
}

/// Reverse a string
///
/// # Example
/// ```tera
/// {{ "hello"|reverse }}
/// ```
/// Output: `olleh`
pub fn reverse(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("reverse filter requires a string")
    })?;
    Ok(Value::String(s.chars().rev().collect()))
}

/// Truncate a string to a specified length
///
/// If the string is longer than the specified length, it will be truncated
/// and "..." will be appended. Uses character count, not byte count.
///
/// # Example
/// ```tera
/// {{ "Hello World"|truncate(length=5) }}
/// ```
/// Output: `Hello...`
pub fn truncate(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("truncate filter requires a string")
    })?;
    let length = args
        .get("length")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| tera::Error::msg("truncate filter requires a 'length' parameter"))? as usize;

    let chars: Vec<char> = s.chars().collect();
    let result = if chars.len() <= length {
        s.to_string()
    } else {
        let truncated: String = chars[..length].iter().collect();
        format!("{}...", truncated)
    };
    Ok(Value::String(result))
}

/// Join a list of strings with a separator
///
/// # Example
/// ```tera
/// {{ items|join(sep=", ") }}
/// ```
pub fn join(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let items = value.as_array().ok_or_else(|| {
        tera::Error::msg("join filter requires an array")
    })?;
    let separator = args
        .get("sep")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("join filter requires a 'sep' parameter"))?;

    let strings: Result<Vec<String>, _> = items
        .iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| tera::Error::msg("join filter requires array of strings"))
        })
        .collect();

    Ok(Value::String(strings?.join(separator)))
}

/// Provide a default value if the input is empty
///
/// # Example
/// ```tera
/// {{ value|default(value="N/A") }}
/// ```
pub fn default(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().unwrap_or("");
    let default_value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("default filter requires a 'value' parameter"))?;

    let result = if s.is_empty() {
        default_value.to_string()
    } else {
        s.to_string()
    };
    Ok(Value::String(result))
}

/// Capitalize the first character of a string
///
/// # Example
/// ```tera
/// {{ "hello world"|capitalize }}
/// ```
/// Output: `Hello world`
pub fn capitalize(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("capitalize filter requires a string")
    })?;
    let mut chars = s.chars();
    let result = match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    };
    Ok(Value::String(result))
}

/// Convert a string to title case
///
/// # Example
/// ```tera
/// {{ "hello world"|title }}
/// ```
/// Output: `Hello World`
pub fn title(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("title filter requires a string")
    })?;
    let result = s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");
    Ok(Value::String(result))
}

/// Get the length of a string
///
/// # Example
/// ```tera
/// {{ "hello"|length }}
/// ```
/// Output: `5`
pub fn length(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("length filter requires a string")
    })?;
    Ok(Value::Number((s.len() as u64).into()))
}

/// Left-pad a string with a character to a specified width
///
/// # Example
/// ```tera
/// {{ "42"|ljust(width=5, fill="0") }}
/// ```
/// Output: `42000`
pub fn ljust(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("ljust filter requires a string")
    })?;
    let width = args
        .get("width")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| tera::Error::msg("ljust filter requires a 'width' parameter"))? as usize;
    let fill_char = args
        .get("fill")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("ljust filter requires a 'fill' parameter"))?;

    let fill = fill_char.chars().next().unwrap_or(' ');
    let result = if s.len() >= width {
        s.to_string()
    } else {
        let padding = width - s.len();
        format!("{}{}", s, fill.to_string().repeat(padding))
    };
    Ok(Value::String(result))
}

/// Right-pad a string with a character to a specified width
///
/// # Example
/// ```tera
/// {{ "42"|rjust(width=5, fill="0") }}
/// ```
/// Output: `00042`
pub fn rjust(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("rjust filter requires a string")
    })?;
    let width = args
        .get("width")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| tera::Error::msg("rjust filter requires a 'width' parameter"))? as usize;
    let fill_char = args
        .get("fill")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("rjust filter requires a 'fill' parameter"))?;

    let fill = fill_char.chars().next().unwrap_or(' ');
    let result = if s.len() >= width {
        s.to_string()
    } else {
        let padding = width - s.len();
        format!("{}{}", fill.to_string().repeat(padding), s)
    };
    Ok(Value::String(result))
}

/// Replace all occurrences of a substring
///
/// # Example
/// ```tera
/// {{ "hello world"|replace(from="world", to="rust") }}
/// ```
/// Output: `hello rust`
pub fn replace(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("replace filter requires a string")
    })?;
    let from = args
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("replace filter requires a 'from' parameter"))?;
    let to = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("replace filter requires a 'to' parameter"))?;

    Ok(Value::String(s.replace(from, to)))
}

/// Split a string by a separator
///
/// # Example
/// ```tera
/// {{ "a,b,c"|split(sep=",") }}
/// ```
pub fn split(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("split filter requires a string")
    })?;
    let separator = args
        .get("sep")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("split filter requires a 'sep' parameter"))?;

    let parts: Vec<Value> = s
        .split(separator)
        .map(|s| Value::String(s.to_string()))
        .collect();
    Ok(Value::Array(parts))
}

/// Strip HTML tags from a string
///
/// # Example
/// ```tera
/// {{ "<p>Hello</p>"|striptags }}
/// ```
/// Output: `Hello`
pub fn striptags(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("striptags filter requires a string")
    })?;
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

    Ok(Value::String(result))
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
