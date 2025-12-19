//! Compile-time URL pattern validation using nom parser combinators
//!
//! This module provides a procedural macro for validating URL patterns at compile time,
//! similar to Django's URL pattern syntax but with Rust's compile-time guarantees.
//!
//! The implementation uses nom parser combinators to build an Abstract Syntax Tree (AST)
//! of the URL pattern, enabling better error messages and future extensibility.

use nom::{
	IResult, Parser,
	branch::alt,
	bytes::complete::{tag, take_while1},
	character::complete::{alpha1, alphanumeric1},
	combinator::{map, recognize, value, verify},
	multi::{many0, many0_count},
	sequence::{delimited, pair, separated_pair},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Error, LitStr, Result,
	parse::{Parse, ParseStream},
};

// ============================================================================
// AST Definitions
// ============================================================================

/// Abstract Syntax Tree for URL patterns
#[derive(Debug, Clone, PartialEq)]
pub struct UrlPatternAst {
	pub segments: Vec<Segment>,
}

/// A segment in a URL pattern (either literal text or a parameter)
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
	/// Literal text in the URL (e.g., "polls/" or "/results")
	Literal(String),
	/// A parameter that captures part of the URL
	Parameter(Parameter),
}

/// A parameter in a URL pattern
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
	/// The name of the parameter (e.g., "id" in {id})
	pub name: String,
	/// Optional type specifier (e.g., "int" in {<int:id>})
	pub type_spec: Option<TypeSpec>,
}

/// Type specifier for parameters
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpec {
	// === Basic types (legacy) ===
	/// Integer type (unsigned, legacy compatibility)
	Int,
	/// String type
	Str,
	/// UUID type
	Uuid,
	/// Slug type (alphanumeric + hyphens/underscores)
	Slug,
	/// Path type (can include slashes)
	Path,

	// === Signed integer types ===
	/// 8-bit signed integer
	I8,
	/// 16-bit signed integer
	I16,
	/// 32-bit signed integer
	I32,
	/// 64-bit signed integer
	I64,

	// === Unsigned integer types ===
	/// 8-bit unsigned integer
	U8,
	/// 16-bit unsigned integer
	U16,
	/// 32-bit unsigned integer
	U32,
	/// 64-bit unsigned integer
	U64,

	// === Floating point types ===
	/// 32-bit floating point
	F32,
	/// 64-bit floating point
	F64,

	// === Other types ===
	/// Boolean type (true/false/1/0)
	Bool,
	/// Email address format
	Email,
	/// ISO 8601 date format (YYYY-MM-DD)
	Date,
}

impl TypeSpec {
	/// Get all valid type specifier names
	///
	/// Returns an array of valid type specifiers for URL parameters:
	/// - `int`: Integer type (unsigned, legacy)
	/// - `str`: String type
	/// - `uuid`: UUID type
	/// - `slug`: Slug type (alphanumeric + hyphens/underscores)
	/// - `path`: Path type (can include slashes)
	/// - `i8`, `i16`, `i32`, `i64`: Signed integer types
	/// - `u8`, `u16`, `u32`, `u64`: Unsigned integer types
	/// - `f32`, `f64`: Floating point types
	/// - `bool`: Boolean type
	/// - `email`: Email address format
	/// - `date`: ISO 8601 date format
	pub fn valid_types() -> &'static [&'static str] {
		&[
			"int", "str", "uuid", "slug", "path", "i8", "i16", "i32", "i64", "u8", "u16", "u32",
			"u64", "f32", "f64", "bool", "email", "date",
		]
	}

	/// Convert from string to TypeSpec
	fn from_str(s: &str) -> Option<Self> {
		match s {
			// Basic types (legacy)
			"int" => Some(TypeSpec::Int),
			"str" => Some(TypeSpec::Str),
			"uuid" => Some(TypeSpec::Uuid),
			"slug" => Some(TypeSpec::Slug),
			"path" => Some(TypeSpec::Path),
			// Signed integers
			"i8" => Some(TypeSpec::I8),
			"i16" => Some(TypeSpec::I16),
			"i32" => Some(TypeSpec::I32),
			"i64" => Some(TypeSpec::I64),
			// Unsigned integers
			"u8" => Some(TypeSpec::U8),
			"u16" => Some(TypeSpec::U16),
			"u32" => Some(TypeSpec::U32),
			"u64" => Some(TypeSpec::U64),
			// Floating point
			"f32" => Some(TypeSpec::F32),
			"f64" => Some(TypeSpec::F64),
			// Other types
			"bool" => Some(TypeSpec::Bool),
			"email" => Some(TypeSpec::Email),
			"date" => Some(TypeSpec::Date),
			_ => None,
		}
	}
}

// ============================================================================
// Nom Parsers
// ============================================================================

/// Parse a valid identifier (starts with letter or underscore, followed by alphanumeric or underscore)
fn identifier(input: &str) -> IResult<&str, &str> {
	recognize(pair(
		alt((alpha1, tag("_"))),
		many0_count(alt((alphanumeric1, tag("_")))),
	))
	.parse(input)
}

/// Parse a type specifier
///
/// Supported types:
/// - Basic: int, str, uuid, slug, path
/// - Signed integers: i8, i16, i32, i64
/// - Unsigned integers: u8, u16, u32, u64
/// - Floating point: f32, f64
/// - Other: bool, email, date
fn type_spec(input: &str) -> IResult<&str, TypeSpec> {
	alt((
		// Basic types (legacy)
		value(TypeSpec::Int, tag("int")),
		value(TypeSpec::Str, tag("str")),
		value(TypeSpec::Uuid, tag("uuid")),
		value(TypeSpec::Slug, tag("slug")),
		value(TypeSpec::Path, tag("path")),
		// Signed integers (order matters: i64 before i6, etc.)
		value(TypeSpec::I64, tag("i64")),
		value(TypeSpec::I32, tag("i32")),
		value(TypeSpec::I16, tag("i16")),
		value(TypeSpec::I8, tag("i8")),
		// Unsigned integers
		value(TypeSpec::U64, tag("u64")),
		value(TypeSpec::U32, tag("u32")),
		value(TypeSpec::U16, tag("u16")),
		value(TypeSpec::U8, tag("u8")),
		// Floating point
		value(TypeSpec::F64, tag("f64")),
		value(TypeSpec::F32, tag("f32")),
		// Other types
		value(TypeSpec::Bool, tag("bool")),
		value(TypeSpec::Email, tag("email")),
		value(TypeSpec::Date, tag("date")),
	))
	.parse(input)
}

/// Parse a typed parameter: <type:name>
fn typed_parameter(input: &str) -> IResult<&str, Parameter> {
	map(
		delimited(
			tag("<"),
			separated_pair(type_spec, tag(":"), identifier),
			tag(">"),
		),
		|(ts, name)| Parameter {
			name: name.to_string(),
			type_spec: Some(ts),
		},
	)
	.parse(input)
}

/// Parse a simple parameter: just an identifier
fn simple_parameter(input: &str) -> IResult<&str, Parameter> {
	map(identifier, |name| Parameter {
		name: name.to_string(),
		type_spec: None,
	})
	.parse(input)
}

/// Parse a parameter: {typed_parameter} or {simple_parameter}
fn parameter(input: &str) -> IResult<&str, Parameter> {
	delimited(tag("{"), alt((typed_parameter, simple_parameter)), tag("}")).parse(input)
}

/// Parse a literal segment (any characters except { and })
fn literal(input: &str) -> IResult<&str, &str> {
	verify(take_while1(|c| c != '{' && c != '}'), |s: &str| {
		!s.is_empty()
	})
	.parse(input)
}

/// Parse a single segment (parameter or literal)
fn segment(input: &str) -> IResult<&str, Segment> {
	alt((
		map(parameter, Segment::Parameter),
		map(literal, |s| Segment::Literal(s.to_string())),
	))
	.parse(input)
}

/// Parse a complete URL pattern
fn url_pattern(input: &str) -> IResult<&str, UrlPatternAst> {
	map(many0(segment), |segments| UrlPatternAst { segments }).parse(input)
}

// ============================================================================
// Error Handling and Validation
// ============================================================================
/// Parse and validate a URL pattern, returning detailed error messages
///
/// This function validates URL patterns at compile time, checking for:
/// - Proper brace matching
/// - Valid parameter names
/// - Correct type specifier syntax
/// - Django-style parameter placement
///
/// Returns an AST representation of the pattern if valid, or a descriptive error message.
pub fn parse_and_validate(pattern: &str) -> std::result::Result<UrlPatternAst, String> {
	// Pre-validation: Check for common errors before parsing
	if pattern.contains("{{") {
		return Err("Nested braces are not allowed in URL patterns. Use single braces like {id}, not {{id}}".to_string());
	}
	if pattern.contains("{}") {
		return Err(
			"Empty parameter name. Parameters must have a name like {id} or {<int:id>}".to_string(),
		);
	}

	// Check for unclosed braces
	let open_count = pattern.chars().filter(|&c| c == '{').count();
	let close_count = pattern.chars().filter(|&c| c == '}').count();
	if open_count != close_count {
		if open_count > close_count {
			if let Some(unclosed_pos) = pattern.find('{') {
				return Err(format!(
					"Unclosed brace in URL pattern. Opening brace at position {} has no matching closing brace",
					unclosed_pos
				));
			}
		} else {
			return Err("Unexpected closing brace. No matching opening brace found.".to_string());
		}
	}

	// Check for Django-style syntax outside braces
	if let Some(pos) = pattern.find('<') {
		// Check if it's inside braces
		let before = &pattern[..pos];
		let open_before = before.chars().filter(|&c| c == '{').count();
		let close_before = before.chars().filter(|&c| c == '}').count();

		if open_before == close_before {
			return Err(format!(
				"Django-style parameters must be inside braces at position {}. Use '{{<type:name>}}' instead of '<type:name>'",
				pos
			));
		}
	}

	// Check for invalid type specifiers
	if let Some(type_start) = pattern.find("<")
		&& let Some(type_end) = pattern[type_start..].find(":")
	{
		let type_spec = &pattern[type_start + 1..type_start + type_end];
		if TypeSpec::from_str(type_spec).is_none() {
			return Err(format!(
				"Invalid type specifier '{}'. Valid types are: {}",
				type_spec,
				TypeSpec::valid_types().join(", ")
			));
		}
	}

	match url_pattern(pattern) {
		Ok((remaining, ast)) => {
			if remaining.is_empty() {
				Ok(ast)
			} else {
				// Calculate position of error
				let error_pos = pattern.len() - remaining.len();

				// Try to give helpful error messages
				if remaining.starts_with('{') {
					Err(format!(
						"Invalid parameter syntax at position {}. Perhaps you meant to close a previous parameter?",
						error_pos
					))
				} else if remaining.starts_with('}') {
					Err(format!(
						"Unexpected closing brace at position {}. No matching opening brace found.",
						error_pos
					))
				} else if remaining.starts_with('<') {
					Err(format!(
						"Django-style parameters must be inside braces at position {}. Use '{{<type:name>}}' instead of '<type:name>'",
						error_pos
					))
				} else {
					Err(format!(
						"Failed to parse URL pattern at position {}: '{}'",
						error_pos, remaining
					))
				}
			}
		}
		Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
			// Try to extract position information
			let remaining = e.input;
			let error_pos = pattern.len() - remaining.len();

			// Check if error is due to invalid type specifier
			if remaining.starts_with('<')
				&& !remaining.starts_with("<int:")
				&& !remaining.starts_with("<str:")
				&& !remaining.starts_with("<uuid:")
				&& !remaining.starts_with("<slug:")
				&& !remaining.starts_with("<path:")
			{
				Err(format!(
					"Invalid type specifier at position {}. Valid types are: {}",
					error_pos,
					TypeSpec::valid_types().join(", ")
				))
			} else {
				Err(format!("Parse error at position {}", error_pos))
			}
		}
		Err(nom::Err::Incomplete(_)) => Err("Incomplete URL pattern".to_string()),
	}
}

// ============================================================================
// Macro Integration
// ============================================================================

/// Parsed URL pattern with validation
struct UrlPattern {
	pattern: String,
	_ast: UrlPatternAst,
}

impl Parse for UrlPattern {
	fn parse(input: ParseStream) -> Result<Self> {
		let pattern_lit: LitStr = input.parse()?;
		let pattern = pattern_lit.value();
		let span = pattern_lit.span();

		// Parse and validate the pattern at compile time
		let ast = parse_and_validate(&pattern).map_err(|e| Error::new(span, e))?;

		Ok(UrlPattern { pattern, _ast: ast })
	}
}
/// Implementation of the `path!` procedural macro
///
/// This function is used internally by the `path!` macro.
/// Users should not call this function directly.
pub fn path_impl(input: TokenStream) -> Result<TokenStream> {
	let pattern: UrlPattern = syn::parse2(input)?;
	let pattern_str = pattern.pattern;

	Ok(quote! {
		#pattern_str
	})
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
	use super::*;

	// AST parsing tests
	#[test]
	fn test_parse_simple_literal() {
		let result = parse_and_validate("polls/");
		let ast = result.unwrap();
		assert_eq!(ast.segments.len(), 1);
		assert!(matches!(ast.segments[0], Segment::Literal(_)));
	}

	#[test]
	fn test_parse_simple_parameter() {
		let result = parse_and_validate("polls/{id}/");
		let ast = result.unwrap();
		assert_eq!(ast.segments.len(), 3);

		match &ast.segments[1] {
			Segment::Parameter(p) => {
				assert_eq!(p.name, "id");
				assert_eq!(p.type_spec, None);
			}
			_ => panic!("Expected parameter segment"),
		}
	}

	#[test]
	fn test_parse_typed_parameter() {
		let result = parse_and_validate("polls/{<int:question_id>}/");
		let ast = result.unwrap();

		// Find the parameter segment
		let param = ast
			.segments
			.iter()
			.find_map(|s| match s {
				Segment::Parameter(p) => Some(p),
				_ => None,
			})
			.expect("Should have parameter segment");

		assert_eq!(param.name, "question_id");
		assert_eq!(param.type_spec, Some(TypeSpec::Int));
	}

	#[test]
	fn test_parse_multiple_parameters() {
		let result = parse_and_validate("users/{user_id}/posts/{post_id}/");
		let ast = result.unwrap();

		let params: Vec<&Parameter> = ast
			.segments
			.iter()
			.filter_map(|s| match s {
				Segment::Parameter(p) => Some(p),
				_ => None,
			})
			.collect();

		assert_eq!(params.len(), 2);
		assert_eq!(params[0].name, "user_id");
		assert_eq!(params[1].name, "post_id");
	}

	// Error case tests
	#[test]
	fn test_invalid_unclosed_brace() {
		let result = parse_and_validate("polls/{id");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Unclosed brace"));
	}

	#[test]
	fn test_invalid_unmatched_closing_brace() {
		let result = parse_and_validate("polls/id}/");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("closing brace"));
	}

	#[test]
	fn test_invalid_empty_param() {
		let result = parse_and_validate("polls/{}/");
		assert!(result.is_err());
		let err = result.unwrap_err();
		eprintln!("Error message: {}", err);
		assert!(err.contains("Empty parameter"));
	}

	#[test]
	fn test_invalid_nested_braces() {
		let result = parse_and_validate("polls/{{id}}/");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Nested braces"));
	}

	#[test]
	fn test_invalid_param_starting_with_number() {
		let result = parse_and_validate("polls/{1id}/");
		assert!(result.is_err());
	}

	#[test]
	fn test_invalid_type_specifier() {
		let result = parse_and_validate("polls/{<invalid:id>}/");
		assert!(result.is_err());
		let err = result.unwrap_err();
		eprintln!("Error message: {}", err);
		assert!(err.contains("Invalid type specifier") || err.contains("Parse error"));
	}

	#[test]
	fn test_reinhardt_style_outside_braces() {
		let result = parse_and_validate("polls/<int:id>/");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("inside braces"));
	}

	// All type specifiers
	#[test]
	fn test_all_type_specifiers() {
		let patterns = vec![
			("polls/{<int:id>}/", TypeSpec::Int),
			("articles/{<str:slug>}/", TypeSpec::Str),
			("objects/{<uuid:id>}/", TypeSpec::Uuid),
			("posts/{<slug:title>}/", TypeSpec::Slug),
			("files/{<path:filepath>}/", TypeSpec::Path),
		];

		for (pattern, expected_type) in patterns {
			let result = parse_and_validate(pattern);
			assert!(result.is_ok(), "Failed to parse: {}", pattern);

			let ast = result.unwrap();
			let param = ast
				.segments
				.iter()
				.find_map(|s| match s {
					Segment::Parameter(p) => Some(p),
					_ => None,
				})
				.expect("Should have parameter");

			assert_eq!(param.type_spec, Some(expected_type));
		}
	}
}
