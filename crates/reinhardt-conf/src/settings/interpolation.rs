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
//! Inside the default/message text of `${VAR:-default}` and
//! `${VAR:?message}`, `$$` is the only special sequence — a lone `$` is
//! preserved literally, and a nested `${...}` is rejected as a syntax
//! error (nested expansion is not supported in v1).
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
	#[error("environment variable `{var}` is required at {path}:{key_path}: {message}")]
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
		map(preceded(tag(":-"), parse_rest_until_brace), |s| {
			Some(Modifier::Default(s))
		}),
		map(preceded(tag(":?"), parse_rest_until_brace), |s| {
			Some(Modifier::RequiredMsg(s))
		}),
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

/// String-template interpolator with injected env lookup.
///
/// Constructed per `TomlFileSource::load()` invocation. The `lookup`
/// closure isolates the interpolator from `std::env`, which keeps unit
/// tests free of `unsafe { env::set_var }` and `#[serial]` requirements.
pub(super) struct Interpolator<'a> {
	lookup: &'a dyn Fn(&str) -> Option<String>,
}

impl<'a> Interpolator<'a> {
	/// Create an interpolator that resolves variables via `lookup`.
	pub(super) fn new(lookup: &'a dyn Fn(&str) -> Option<String>) -> Self {
		Self { lookup }
	}

	/// Resolve a single template string. Returns the substituted result
	/// or an `InterpolationError` whose `path` and `key_path` fields are
	/// left empty — the caller (`interpolate_value`) fills them in.
	pub(super) fn interpolate_str(&self, input: &str) -> Result<String, InterpolationError> {
		let (_rest, segments) = parse_template(input).map_err(|e| InterpolationError::Syntax {
			detail: format!("{}", e),
			snippet: input.to_string(),
			path: PathBuf::new(),
			key_path: String::new(),
		})?;

		let mut out = String::with_capacity(input.len());
		for segment in segments {
			match segment {
				Segment::Literal(s) => out.push_str(&s),
				Segment::Required { var } => match self.resolve(&var) {
					Some(v) => out.push_str(&v),
					None => {
						return Err(InterpolationError::Required {
							var,
							path: PathBuf::new(),
							key_path: String::new(),
						});
					}
				},
				Segment::Default { var, default } => match self.resolve(&var) {
					Some(v) => out.push_str(&v),
					None => out.push_str(&default),
				},
				Segment::RequiredMsg { var, message } => match self.resolve(&var) {
					Some(v) => out.push_str(&v),
					None => {
						return Err(InterpolationError::RequiredWithMessage {
							var,
							message,
							path: PathBuf::new(),
							key_path: String::new(),
						});
					}
				},
			}
		}
		Ok(out)
	}

	/// Look up `name`. Returns `None` for both unset variables and empty
	/// strings, enforcing the strict-empty invariant.
	fn resolve(&self, name: &str) -> Option<String> {
		match (self.lookup)(name) {
			Some(s) if !s.is_empty() => Some(s),
			_ => None,
		}
	}
}

use std::path::Path;

/// One segment of a TOML key path — either a table key or an array index.
#[derive(Debug, Clone)]
enum KeyPathSegment {
	Key(String),
	Index(usize),
}

/// Stack of key-path segments accumulated during the AST walk.
#[derive(Default)]
struct KeyPath(Vec<KeyPathSegment>);

impl KeyPath {
	fn push_key(&mut self, key: &str) {
		self.0.push(KeyPathSegment::Key(key.to_string()));
	}

	fn push_index(&mut self, idx: usize) {
		self.0.push(KeyPathSegment::Index(idx));
	}

	fn pop(&mut self) {
		self.0.pop();
	}
}

impl std::fmt::Display for KeyPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (i, seg) in self.0.iter().enumerate() {
			match seg {
				KeyPathSegment::Key(k) if i == 0 => write!(f, "{}", k)?,
				KeyPathSegment::Key(k) => write!(f, ".{}", k)?,
				KeyPathSegment::Index(idx) => write!(f, "[{}]", idx)?,
			}
		}
		Ok(())
	}
}

impl<'a> Interpolator<'a> {
	/// Recursively interpolate every `toml::Value::String` in the tree.
	/// `toml_path` is recorded in errors for diagnostics.
	pub(super) fn interpolate_value(
		&self,
		value: &mut toml::Value,
		toml_path: &Path,
	) -> Result<(), InterpolationError> {
		let mut key_path = KeyPath::default();
		self.walk(value, toml_path, &mut key_path)
	}

	fn walk(
		&self,
		value: &mut toml::Value,
		toml_path: &Path,
		key_path: &mut KeyPath,
	) -> Result<(), InterpolationError> {
		match value {
			toml::Value::String(s) => match self.interpolate_str(s) {
				Ok(replaced) => {
					*s = replaced;
					Ok(())
				}
				Err(err) => Err(attach_context(err, toml_path, key_path)),
			},
			toml::Value::Table(table) => {
				for (k, child) in table.iter_mut() {
					key_path.push_key(k);
					self.walk(child, toml_path, key_path)?;
					key_path.pop();
				}
				Ok(())
			}
			toml::Value::Array(arr) => {
				for (idx, child) in arr.iter_mut().enumerate() {
					key_path.push_index(idx);
					self.walk(child, toml_path, key_path)?;
					key_path.pop();
				}
				Ok(())
			}
			// Numeric, boolean, datetime — left untouched per the
			// type-bounded scope invariant.
			_ => Ok(()),
		}
	}
}

/// Replace empty `path` / `key_path` fields on an error produced by
/// `interpolate_str` with the surrounding TOML context.
fn attach_context(
	err: InterpolationError,
	toml_path: &Path,
	key_path: &KeyPath,
) -> InterpolationError {
	let path = toml_path.to_path_buf();
	let key_path_str = key_path.to_string();
	match err {
		InterpolationError::Required { var, .. } => InterpolationError::Required {
			var,
			path,
			key_path: key_path_str,
		},
		InterpolationError::RequiredWithMessage { var, message, .. } => {
			InterpolationError::RequiredWithMessage {
				var,
				message,
				path,
				key_path: key_path_str,
			}
		}
		InterpolationError::Syntax {
			detail, snippet, ..
		} => InterpolationError::Syntax {
			detail,
			snippet,
			path,
			key_path: key_path_str,
		},
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
		let _def = Segment::Default {
			var: "V".into(),
			default: "d".into(),
		};
		let _msg = Segment::RequiredMsg {
			var: "V".into(),
			message: "m".into(),
		};
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
	#[case::lone_dollar_in_default(
		"${VAR:-foo$bar}",
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

		// Assert — every case enters parse_placeholder (consumes `${`),
		// so failures MUST propagate as `nom::Err::Failure` due to the
		// `cut` boundary. Asserting `Err::Failure` specifically locks
		// in the cut-driven semantics: a future refactor that drops
		// `cut` would surface here as a regression.
		assert!(
			matches!(result, Err(nom::Err::Failure(_))),
			"input `{}` should produce Err::Failure (cut-protected), got {:?}",
			input,
			result,
		);
	}

	// --- interpolate_str: positive and error cases ------------------------

	use std::collections::HashMap;

	macro_rules! envmap {
		( $( $k:expr => $v:expr ),* $(,)? ) => {{
			// `mut` is unused when the macro is invoked with zero pairs, but
			// is required for the variadic case. Allow it locally to keep
			// the macro single-arm.
			#[allow(unused_mut)]
			let mut m: HashMap<&'static str, &'static str> = HashMap::new();
			$( m.insert($k, $v); )*
			m
		}};
	}

	#[rstest]
	#[case::set_value(envmap!{"VAR" => "value"}, "${VAR}", "value")]
	#[case::with_prefix_suffix(envmap!{"H" => "ex"}, "http://${H}/x", "http://ex/x")]
	#[case::default_used(envmap!{}, "${VAR:-fallback}", "fallback")]
	#[case::default_overridden(envmap!{"V" => "x"}, "${V:-fallback}", "x")]
	#[case::empty_default_explicit(envmap!{}, "${VAR:-}", "")]
	#[case::empty_env_treated_as_unset(envmap!{"V" => ""}, "${V:-fb}", "fb")]
	#[case::escape_passthrough(envmap!{}, "$$50", "$50")]
	fn interpolate_str_ok(
		#[case] env: HashMap<&'static str, &'static str>,
		#[case] tmpl: &str,
		#[case] expected: &str,
	) {
		// Arrange
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let result = interp
			.interpolate_str(tmpl)
			.expect("interpolation should succeed");

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	fn interpolate_str_required_unset_returns_required_error() {
		// Arrange
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp.interpolate_str("${MISSING}").unwrap_err();

		// Assert
		assert!(matches!(
			&err,
			InterpolationError::Required { var, .. } if var == "MISSING"
		));
	}

	#[rstest]
	fn interpolate_str_required_empty_returns_required_error() {
		// Arrange — empty string is treated as unset (strict semantics)
		let env = envmap! {"V" => ""};
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp.interpolate_str("${V}").unwrap_err();

		// Assert
		assert!(matches!(err, InterpolationError::Required { .. }));
	}

	#[rstest]
	fn interpolate_str_required_msg_returns_message() {
		// Arrange
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp.interpolate_str("${P:?Set via direnv}").unwrap_err();

		// Assert
		assert!(matches!(
			&err,
			InterpolationError::RequiredWithMessage { var, message, .. }
				if var == "P" && message == "Set via direnv"
		));
	}

	#[rstest]
	fn interpolate_str_syntax_error_for_unclosed_brace() {
		// Arrange
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp.interpolate_str("${UNCLOSED").unwrap_err();

		// Assert
		assert!(matches!(err, InterpolationError::Syntax { .. }));
	}

	// --- interpolate_value: AST walker ------------------------------------

	#[rstest]
	fn interpolate_value_walks_nested_table() {
		// Arrange
		let mut value: toml::Value = toml::from_str(
			r#"
			[database]
			host = "${DB_HOST}"
			port = 5432
			"#,
		)
		.unwrap();
		let env = envmap! {"DB_HOST" => "postgres"};
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		interp
			.interpolate_value(&mut value, Path::new("test.toml"))
			.expect("walk should succeed");

		// Assert
		assert_eq!(value["database"]["host"].as_str(), Some("postgres"));
		assert_eq!(value["database"]["port"].as_integer(), Some(5432));
	}

	#[rstest]
	fn interpolate_value_propagates_key_path_in_error() {
		// Arrange
		let mut value: toml::Value = toml::from_str(
			r#"
			[core.databases.default]
			host = "${MISSING_VAR}"
			"#,
		)
		.unwrap();
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp
			.interpolate_value(&mut value, Path::new("local.toml"))
			.unwrap_err();

		// Assert
		let msg = err.to_string();
		assert!(msg.contains("local.toml"), "msg = {}", msg);
		assert!(msg.contains("core.databases.default.host"), "msg = {}", msg);
		assert!(msg.contains("MISSING_VAR"), "msg = {}", msg);
	}

	#[rstest]
	fn interpolate_value_walks_array_with_index_in_path() {
		// Arrange
		let mut value: toml::Value = toml::from_str(
			r#"
			services = ["${SVC_A}", "${SVC_B:-default-b}"]
			"#,
		)
		.unwrap();
		let env = envmap! {"SVC_A" => "alpha"};
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		interp
			.interpolate_value(&mut value, Path::new("test.toml"))
			.unwrap();

		// Assert
		let arr = value["services"].as_array().unwrap();
		assert_eq!(arr[0].as_str(), Some("alpha"));
		assert_eq!(arr[1].as_str(), Some("default-b"));
	}

	#[rstest]
	fn interpolate_value_array_error_includes_index_in_path() {
		// Arrange
		let mut value: toml::Value = toml::from_str(r#"services = ["${MISSING_SVC}"]"#).unwrap();
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		let err = interp
			.interpolate_value(&mut value, Path::new("test.toml"))
			.unwrap_err();

		// Assert
		let msg = err.to_string();
		assert!(msg.contains("services[0]"), "msg = {}", msg);
	}

	#[rstest]
	fn interpolate_value_does_not_recurse_into_resolved_value() {
		// Arrange — VAR resolves to "${INNER}", which must NOT be re-expanded
		let mut value = toml::Value::String("${OUTER}".to_string());
		let env = envmap! {"OUTER" => "${INNER}"};
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		interp
			.interpolate_value(&mut value, Path::new("x.toml"))
			.unwrap();

		// Assert — single-pass invariant
		assert_eq!(value.as_str(), Some("${INNER}"));
	}

	#[rstest]
	fn interpolate_value_skips_non_string_types() {
		// Arrange
		let mut value: toml::Value = toml::from_str(
			r#"
			port = 5432
			enabled = true
			rate = 1.5
			"#,
		)
		.unwrap();
		let env: HashMap<&'static str, &'static str> = HashMap::new();
		let lookup = |n: &str| env.get(n).map(|s| s.to_string());
		let interp = Interpolator::new(&lookup);

		// Act
		interp
			.interpolate_value(&mut value, Path::new("x.toml"))
			.unwrap();

		// Assert — non-string types passthrough
		assert_eq!(value["port"].as_integer(), Some(5432));
		assert_eq!(value["enabled"].as_bool(), Some(true));
		assert_eq!(value["rate"].as_float(), Some(1.5));
	}
}
