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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("upper filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("lower filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("trim filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("reverse filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("truncate filter requires a string"))?;
	let length = args
		.get("length")
		.and_then(|v| v.as_u64())
		.ok_or_else(|| tera::Error::msg("truncate filter requires a 'length' parameter"))?
		as usize;

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
	let items = value
		.as_array()
		.ok_or_else(|| tera::Error::msg("join filter requires an array"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("capitalize filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("title filter requires a string"))?;
	let result = s
		.split_whitespace()
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("length filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("ljust filter requires a string"))?;
	let width =
		args.get("width")
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("rjust filter requires a string"))?;
	let width =
		args.get("width")
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("replace filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("split filter requires a string"))?;
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
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("striptags filter requires a string"))?;
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
		let value = Value::String("hello".to_string());
		let args = HashMap::new();
		assert_eq!(
			upper(&value, &args).unwrap(),
			Value::String("HELLO".to_string())
		);

		let value2 = Value::String("".to_string());
		assert_eq!(
			upper(&value2, &args).unwrap(),
			Value::String("".to_string())
		);

		let value3 = Value::String("HeLLo".to_string());
		assert_eq!(
			upper(&value3, &args).unwrap(),
			Value::String("HELLO".to_string())
		);
	}

	#[test]
	fn test_lower() {
		let value = Value::String("HELLO".to_string());
		let args = HashMap::new();
		assert_eq!(
			lower(&value, &args).unwrap(),
			Value::String("hello".to_string())
		);

		let value2 = Value::String("".to_string());
		assert_eq!(
			lower(&value2, &args).unwrap(),
			Value::String("".to_string())
		);

		let value3 = Value::String("HeLLo".to_string());
		assert_eq!(
			lower(&value3, &args).unwrap(),
			Value::String("hello".to_string())
		);
	}

	#[test]
	fn test_trim() {
		let value = Value::String("  hello  ".to_string());
		let args = HashMap::new();
		assert_eq!(
			trim(&value, &args).unwrap(),
			Value::String("hello".to_string())
		);

		let value2 = Value::String("hello".to_string());
		assert_eq!(
			trim(&value2, &args).unwrap(),
			Value::String("hello".to_string())
		);

		let value3 = Value::String("  ".to_string());
		assert_eq!(trim(&value3, &args).unwrap(), Value::String("".to_string()));
	}

	#[test]
	fn test_reverse() {
		let value = Value::String("hello".to_string());
		let args = HashMap::new();
		assert_eq!(
			reverse(&value, &args).unwrap(),
			Value::String("olleh".to_string())
		);

		let value2 = Value::String("".to_string());
		assert_eq!(
			reverse(&value2, &args).unwrap(),
			Value::String("".to_string())
		);

		let value3 = Value::String("abc".to_string());
		assert_eq!(
			reverse(&value3, &args).unwrap(),
			Value::String("cba".to_string())
		);
	}

	#[test]
	fn test_truncate() {
		let value = Value::String("Hello World".to_string());
		let mut args = HashMap::new();
		args.insert("length".to_string(), Value::Number(5.into()));
		assert_eq!(
			truncate(&value, &args).unwrap(),
			Value::String("Hello...".to_string())
		);

		let value2 = Value::String("Hi".to_string());
		let mut args2 = HashMap::new();
		args2.insert("length".to_string(), Value::Number(5.into()));
		assert_eq!(
			truncate(&value2, &args2).unwrap(),
			Value::String("Hi".to_string())
		);

		let value3 = Value::String("".to_string());
		let mut args3 = HashMap::new();
		args3.insert("length".to_string(), Value::Number(5.into()));
		assert_eq!(
			truncate(&value3, &args3).unwrap(),
			Value::String("".to_string())
		);
	}

	#[test]
	fn test_join() {
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

		let empty_items: Vec<Value> = vec![];
		let value2 = Value::Array(empty_items);
		assert_eq!(join(&value2, &args).unwrap(), Value::String("".to_string()));
	}

	#[test]
	fn test_default() {
		let value = Value::String("hello".to_string());
		let mut args = HashMap::new();
		args.insert("value".to_string(), Value::String("N/A".to_string()));
		assert_eq!(
			default(&value, &args).unwrap(),
			Value::String("hello".to_string())
		);

		let value2 = Value::String("".to_string());
		assert_eq!(
			default(&value2, &args).unwrap(),
			Value::String("N/A".to_string())
		);
	}

	#[test]
	fn test_capitalize() {
		let value = Value::String("hello".to_string());
		let args = HashMap::new();
		assert_eq!(
			capitalize(&value, &args).unwrap(),
			Value::String("Hello".to_string())
		);

		let value2 = Value::String("HELLO".to_string());
		assert_eq!(
			capitalize(&value2, &args).unwrap(),
			Value::String("HELLO".to_string())
		);

		let value3 = Value::String("".to_string());
		assert_eq!(
			capitalize(&value3, &args).unwrap(),
			Value::String("".to_string())
		);
	}

	#[test]
	fn test_title() {
		let value = Value::String("hello world".to_string());
		let args = HashMap::new();
		assert_eq!(
			title(&value, &args).unwrap(),
			Value::String("Hello World".to_string())
		);

		let value2 = Value::String("".to_string());
		assert_eq!(
			title(&value2, &args).unwrap(),
			Value::String("".to_string())
		);

		let value3 = Value::String("a b c".to_string());
		assert_eq!(
			title(&value3, &args).unwrap(),
			Value::String("A B C".to_string())
		);
	}

	#[test]
	fn test_length() {
		let value = Value::String("hello".to_string());
		let args = HashMap::new();
		assert_eq!(length(&value, &args).unwrap(), Value::Number(5.into()));

		let value2 = Value::String("".to_string());
		assert_eq!(length(&value2, &args).unwrap(), Value::Number(0.into()));
	}

	#[test]
	fn test_ljust() {
		let value = Value::String("42".to_string());
		let mut args = HashMap::new();
		args.insert("width".to_string(), Value::Number(5.into()));
		args.insert("fill".to_string(), Value::String("0".to_string()));
		assert_eq!(
			ljust(&value, &args).unwrap(),
			Value::String("42000".to_string())
		);

		let value2 = Value::String("hello".to_string());
		let mut args2 = HashMap::new();
		args2.insert("width".to_string(), Value::Number(3.into()));
		args2.insert("fill".to_string(), Value::String("0".to_string()));
		assert_eq!(
			ljust(&value2, &args2).unwrap(),
			Value::String("hello".to_string())
		);
	}

	#[test]
	fn test_rjust() {
		let value = Value::String("42".to_string());
		let mut args = HashMap::new();
		args.insert("width".to_string(), Value::Number(5.into()));
		args.insert("fill".to_string(), Value::String("0".to_string()));
		assert_eq!(
			rjust(&value, &args).unwrap(),
			Value::String("00042".to_string())
		);

		let value2 = Value::String("hello".to_string());
		let mut args2 = HashMap::new();
		args2.insert("width".to_string(), Value::Number(3.into()));
		args2.insert("fill".to_string(), Value::String("0".to_string()));
		assert_eq!(
			rjust(&value2, &args2).unwrap(),
			Value::String("hello".to_string())
		);
	}

	#[test]
	fn test_replace() {
		let value = Value::String("hello world".to_string());
		let mut args = HashMap::new();
		args.insert("from".to_string(), Value::String("world".to_string()));
		args.insert("to".to_string(), Value::String("rust".to_string()));
		assert_eq!(
			replace(&value, &args).unwrap(),
			Value::String("hello rust".to_string())
		);

		let value2 = Value::String("".to_string());
		let mut args2 = HashMap::new();
		args2.insert("from".to_string(), Value::String("a".to_string()));
		args2.insert("to".to_string(), Value::String("b".to_string()));
		assert_eq!(
			replace(&value2, &args2).unwrap(),
			Value::String("".to_string())
		);
	}

	#[test]
	fn test_split() {
		let value = Value::String("a,b,c".to_string());
		let mut args = HashMap::new();
		args.insert("sep".to_string(), Value::String(",".to_string()));
		let result = split(&value, &args).unwrap();
		let expected = Value::Array(vec![
			Value::String("a".to_string()),
			Value::String("b".to_string()),
			Value::String("c".to_string()),
		]);
		assert_eq!(result, expected);

		let value2 = Value::String("hello".to_string());
		let result2 = split(&value2, &args).unwrap();
		let expected2 = Value::Array(vec![Value::String("hello".to_string())]);
		assert_eq!(result2, expected2);
	}

	#[test]
	fn test_striptags() {
		let value = Value::String("<p>Hello</p>".to_string());
		let args = HashMap::new();
		assert_eq!(
			striptags(&value, &args).unwrap(),
			Value::String("Hello".to_string())
		);

		let value2 = Value::String("No tags".to_string());
		assert_eq!(
			striptags(&value2, &args).unwrap(),
			Value::String("No tags".to_string())
		);

		let value3 = Value::String("<div><p>Nested</p></div>".to_string());
		assert_eq!(
			striptags(&value3, &args).unwrap(),
			Value::String("Nested".to_string())
		);
	}
}
