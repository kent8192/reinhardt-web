//! TOML configuration interpolation.
//!
//! When [`TomlFileSource::with_interpolation(true)`] is set, every TOML
//! `Value::String` is scanned for the following tokens before the value
//! is merged into the configuration:
//!
//! | Token              | Behavior                                         |
//! |--------------------|--------------------------------------------------|
//! | `${VAR}`           | required — fails if `VAR` is unset OR empty      |
//! | `${VAR:-default}`  | substitutes `default` if `VAR` is unset OR empty |
//! | `${VAR:-}`         | explicit empty fallback                          |
//! | `${VAR:?message}`  | fails with `message` if `VAR` is unset OR empty  |
//! | `$$`               | escape — emits a literal `$`                     |
//!
//! ## Invariants
//!
//! 1. **Single-pass expansion** — Resolved values are not re-expanded.
//! 2. **Strict empty handling** — `lookup` returning `Some("")` is treated
//!    as `None` so `${VAR}` and `${VAR:-default}` apply uniformly.
//! 3. **Fail-fast** — Any failure aborts `TomlFileSource::load()`.
//! 4. **Type-bounded scope** — Only `toml::Value::String` is rewritten.
//!
//! [`TomlFileSource::with_interpolation(true)`]: super::sources::TomlFileSource::with_interpolation
//!
//! The parser AST and helpers are introduced ahead of the evaluator and
//! `TomlFileSource` wire-up. They are exercised by this module's own tests
//! but have no non-test caller yet, so `dead_code` would fire on every
//! public-but-internal helper. The follow-up commit (`Interpolator` +
//! `walk_value`) consumes them and the allow can be removed at that point.
#![allow(dead_code)]

use std::path::PathBuf;

/// A parsed segment of an interpolation template.
///
/// `parse_template` produces a sequence of these. `Literal` runs may be
/// adjacent because `$$` produces its own segment; the eval step
/// concatenates them.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Segment {
	/// Literal text — emit as-is. `$$` escapes are already collapsed.
	Literal(String),
	/// `${VAR}` — fails if `VAR` is unset or empty.
	Required { var: String },
	/// `${VAR:-default}` — substitutes `default` if `VAR` is unset or empty.
	Default { var: String, default: String },
	/// `${VAR:?message}` — fails with `message` if `VAR` is unset or empty.
	RequiredMsg { var: String, message: String },
}

/// Errors produced by TOML interpolation.
///
/// Every variant carries file path and TOML key path context so callers
/// can locate failures without re-reading the source file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InterpolationError {
	/// A required `${VAR}` token resolved to an unset or empty environment
	/// variable.
	#[error(
		"environment variable `{var}` is required at {path}:{key_path} \
		 but is unset or empty in the process environment"
	)]
	Required {
		/// Variable name as it appeared in the template.
		var: String,
		/// Path of the TOML file that contained the failing template.
		path: PathBuf,
		/// Dot/bracket-separated TOML key path (e.g., `core.databases.default.host`).
		key_path: String,
	},

	/// A `${VAR:?message}` token failed; `message` is the user-supplied hint.
	#[error(
		"environment variable `{var}` is required at {path}:{key_path}: {message}"
	)]
	RequiredWithMessage {
		/// Variable name.
		var: String,
		/// User-supplied error message from the `:?` modifier.
		message: String,
		/// File path.
		path: PathBuf,
		/// Key path.
		key_path: String,
	},

	/// The interpolation template could not be parsed.
	#[error(
		"invalid interpolation syntax at {path}:{key_path}: {detail} \
		 (offending value: `{snippet}`)"
	)]
	Syntax {
		/// Human-readable parser diagnostic.
		detail: String,
		/// The offending template string (untruncated; usually short).
		snippet: String,
		/// File path.
		path: PathBuf,
		/// Key path.
		key_path: String,
	},
}

use nom::{
	IResult, Parser,
	branch::alt,
	bytes::complete::{tag, take_while, take_while1},
	character::complete::satisfy,
	combinator::{cut, map, recognize},
	multi::many0,
	sequence::preceded,
};

/// Top-level template parser.
pub(super) fn parse_template(input: &str) -> IResult<&str, Vec<Segment>> {
	many0(alt((
		// $$ → literal $ (must come before $ alone)
		map(tag("$$"), |_| Segment::Literal("$".to_string())),
		// ${...} placeholder (commits via `cut` once "${" is seen)
		parse_placeholder,
		// run of non-$ chars
		map(parse_literal_run, Segment::Literal),
		// lone $ NOT followed by { (since ${ is captured above)
		map(tag("$"), |_| Segment::Literal("$".to_string())),
	)))
	.parse(input)
}

/// Internal modifier representation, used during placeholder parsing.
enum Modifier {
	Default(String),
	RequiredMsg(String),
}

/// Consume one or more chars that are not `$`.
fn parse_literal_run(input: &str) -> IResult<&str, String> {
	let (rest, s) = take_while1(|c: char| c != '$')(input)?;
	Ok((rest, s.to_string()))
}

/// Parse a `${...}` placeholder. Once `${` is consumed, `cut` commits;
/// any further failure becomes a hard error rather than triggering
/// `alt` rollback.
fn parse_placeholder(input: &str) -> IResult<&str, Segment> {
	let (input, _) = tag("${")(input)?;
	cut(parse_placeholder_body).parse(input)
}

fn parse_placeholder_body(input: &str) -> IResult<&str, Segment> {
	let (input, var_name) = parse_var_name(input)?;
	let var = var_name.to_string();
	let (input, modifier) = parse_modifier(input)?;
	let (input, _) = tag("}")(input)?;

	let segment = match modifier {
		None => Segment::Required { var },
		Some(Modifier::Default(default)) => Segment::Default { var, default },
		Some(Modifier::RequiredMsg(message)) => Segment::RequiredMsg { var, message },
	};
	Ok((input, segment))
}

/// Variable name: `[A-Za-z_][A-Za-z0-9_]*`.
fn parse_var_name(input: &str) -> IResult<&str, &str> {
	recognize((
		satisfy(|c: char| c.is_ascii_alphabetic() || c == '_'),
		take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
	))
	.parse(input)
}

/// Optional modifier after the variable name: `:-default` or `:?message`.
fn parse_modifier(input: &str) -> IResult<&str, Option<Modifier>> {
	alt((
		map(preceded(tag(":-"), parse_rest_until_brace), |s| Some(Modifier::Default(s))),
		map(preceded(tag(":?"), parse_rest_until_brace), |s| Some(Modifier::RequiredMsg(s))),
		// No modifier: nothing consumed.
		parse_no_modifier,
	))
	.parse(input)
}

/// No-op modifier branch: consumes nothing, yields `None`.
fn parse_no_modifier(input: &str) -> IResult<&str, Option<Modifier>> {
	Ok((input, None))
}

/// Consume default/message text up to the closing `}`.
/// Processes `$$` as literal `$`. Nested `${...}` are not supported and
/// will trigger the `cut` boundary in `parse_placeholder` if encountered.
fn parse_rest_until_brace(input: &str) -> IResult<&str, String> {
	let mut out = String::new();
	let mut chars = input;

	loop {
		if chars.is_empty() {
			return Err(nom::Err::Error(nom::error::Error::new(
				chars,
				nom::error::ErrorKind::Eof,
			)));
		}
		if let Some(rest) = chars.strip_prefix("$$") {
			out.push('$');
			chars = rest;
			continue;
		}
		// Reject nested `${` so the caller's `cut` surfaces a syntax error.
		if chars.starts_with("${") {
			return Err(nom::Err::Error(nom::error::Error::new(
				chars,
				nom::error::ErrorKind::Char,
			)));
		}
		if chars.starts_with('}') {
			return Ok((chars, out));
		}
		let mut iter = chars.chars();
		let c = iter.next().unwrap();
		out.push(c);
		chars = iter.as_str();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::path::PathBuf;

	// --- existing smoke tests retained -----------------------------------

	#[test]
	fn segment_variants_construct() {
		let _lit = Segment::Literal("x".into());
		let _req = Segment::Required { var: "V".into() };
		let _def = Segment::Default { var: "V".into(), default: "d".into() };
		let _msg = Segment::RequiredMsg { var: "V".into(), message: "m".into() };
	}

	#[test]
	fn interpolation_error_displays_context() {
		let err = InterpolationError::Required {
			var: "DB_HOST".into(),
			path: PathBuf::from("local.toml"),
			key_path: "database.host".into(),
		};
		let msg = err.to_string();
		assert!(msg.contains("DB_HOST"));
		assert!(msg.contains("local.toml"));
		assert!(msg.contains("database.host"));
	}

	// --- parser positive cases --------------------------------------------

	#[rstest]
	#[case::empty("", vec![])]
	#[case::literal_only("hello", vec![Segment::Literal("hello".into())])]
	#[case::escape_only("$$", vec![Segment::Literal("$".into())])]
	#[case::required("${VAR}", vec![Segment::Required { var: "VAR".into() }])]
	#[case::default_empty(
		"${VAR:-}",
		vec![Segment::Default { var: "VAR".into(), default: "".into() }]
	)]
	#[case::default_with_value(
		"${VAR:-localhost}",
		vec![Segment::Default { var: "VAR".into(), default: "localhost".into() }]
	)]
	#[case::required_msg(
		"${VAR:?Set me}",
		vec![Segment::RequiredMsg { var: "VAR".into(), message: "Set me".into() }]
	)]
	#[case::two_placeholders(
		"${A}${B}",
		vec![
			Segment::Required { var: "A".into() },
			Segment::Required { var: "B".into() },
		]
	)]
	#[case::placeholder_with_literal(
		"a${A}-${B}b",
		vec![
			Segment::Literal("a".into()),
			Segment::Required { var: "A".into() },
			Segment::Literal("-".into()),
			Segment::Required { var: "B".into() },
			Segment::Literal("b".into()),
		]
	)]
	#[case::escape_then_brace(
		"$${VAR}",
		vec![
			Segment::Literal("$".into()),
			Segment::Literal("{VAR}".into()),
		]
	)]
	#[case::var_with_digits(
		"${VAR_123}",
		vec![Segment::Required { var: "VAR_123".into() }]
	)]
	#[case::dollar_then_letter(
		"$abc",
		vec![
			Segment::Literal("$".into()),
			Segment::Literal("abc".into()),
		]
	)]
	#[case::escape_inside_default(
		"${VAR:-foo$$bar}",
		vec![Segment::Default { var: "VAR".into(), default: "foo$bar".into() }]
	)]
	fn parse_template_ok(#[case] input: &str, #[case] expected: Vec<Segment>) {
		// Arrange / Act
		let result = parse_template(input);

		// Assert
		let (rest, segments) = result.expect("parse should succeed");
		assert_eq!(rest, "", "parser left unconsumed input");
		assert_eq!(segments, expected);
	}

	// --- parser negative cases --------------------------------------------

	#[rstest]
	#[case::unclosed_brace("${VAR")]
	#[case::empty_var_name("${}")]
	#[case::var_starts_with_digit("${1VAR}")]
	#[case::var_with_hyphen("${MY-VAR}")]
	#[case::single_colon("${VAR:default}")]
	#[case::dash_only("${VAR-default}")]
	#[case::nested_placeholder("${A:-${B}}")]
	#[case::question_only("${VAR?msg}")]
	fn parse_template_err(#[case] input: &str) {
		// Arrange / Act
		let result = parse_template(input);

		// Assert
		match result {
			Err(_) => {}
			Ok((rest, _segments)) => {
				assert_ne!(rest, "", "input `{}` parsed successfully but should fail", input);
			}
		}
	}
}
