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

#[cfg(test)]
mod smoke_tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn segment_variants_construct() {
		// Arrange / Act
		let _lit = Segment::Literal("x".into());
		let _req = Segment::Required { var: "V".into() };
		let _def = Segment::Default { var: "V".into(), default: "d".into() };
		let _msg = Segment::RequiredMsg { var: "V".into(), message: "m".into() };

		// Assert — compile-time check only
	}

	#[test]
	fn interpolation_error_displays_context() {
		// Arrange
		let err = InterpolationError::Required {
			var: "DB_HOST".into(),
			path: PathBuf::from("local.toml"),
			key_path: "database.host".into(),
		};

		// Act
		let msg = err.to_string();

		// Assert
		assert!(msg.contains("DB_HOST"));
		assert!(msg.contains("local.toml"));
		assert!(msg.contains("database.host"));
	}
}
