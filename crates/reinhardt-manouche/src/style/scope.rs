//! Deterministic component-style scope identities and generated names.

use std::fmt::Write as _;

use proc_macro2::Span;
use sha2::{Digest, Sha256};

use crate::{StyleRuntimeType, StyleVariableConstraint, TypedValueExpr};

/// Cargo and generated-type inputs that uniquely identify one component style definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleCompileContext<'a> {
	/// Selected Cargo package name.
	pub package_name: &'a str,
	/// Selected Cargo package version.
	pub package_version: &'a str,
	/// Authored generated style type name shared by macro expansion and CSS extraction.
	pub style_type_name: &'a str,
}

/// Stable scope identity and its public twelve-hex SHA-256 suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleScope {
	/// Exact NUL-separated versioned identity string.
	pub identity: String,
	/// First twelve lowercase hexadecimal characters of the identity's SHA-256 digest.
	pub suffix: String,
}

/// Generated class metadata consumed by Rust code generation and CSS lowering.
#[derive(Debug, Clone)]
pub struct ScopedClass {
	/// CSS class spelling exactly as authored.
	pub authored_name: String,
	/// Generated ordinary Rust accessor identifier text.
	pub accessor: String,
	/// Final scoped CSS class name.
	pub css_name: String,
	/// Span of the authored class selector.
	pub span: Span,
}

/// Generated component-variable metadata consumed by Rust code generation.
#[derive(Debug, Clone)]
pub struct ScopedVariable {
	/// Component-variable name exactly as authored.
	pub authored_name: String,
	/// Final scoped CSS custom-property name.
	pub custom_property_name: String,
	/// Closed runtime wrapper category for generated setters.
	pub runtime_type: StyleRuntimeType,
	/// Numeric constraint generated setters must enforce, when one is required.
	pub runtime_constraint: Option<StyleVariableConstraint>,
	/// Zero-based source position in the authored `vars` block.
	pub source_index: usize,
	/// Type-checked authored default expression.
	pub default: TypedValueExpr,
	/// Span of the authored variable declaration.
	pub span: Span,
}

impl StyleScope {
	/// Computes the versioned scope identity from its complete normative input tuple.
	pub fn new(context: &StyleCompileContext<'_>) -> Self {
		let identity = format!(
			"rstyle-v2\0{}\0{}\0{}",
			context.package_name, context.package_version, context.style_type_name
		);
		let digest = Sha256::digest(identity.as_bytes());
		let mut suffix = String::with_capacity(12);
		for byte in &digest[..6] {
			write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
		}
		Self { identity, suffix }
	}

	/// Generates one scoped CSS class name from a validated local class.
	pub(crate) fn class_name(&self, local_name: &str) -> String {
		format!("{local_name}--rs-{}", self.suffix)
	}

	/// Generates one scoped custom-property name from a validated kebab-case variable name.
	pub(crate) fn variable_name(&self, variable_name: &str) -> String {
		format!("--rs-{}-{variable_name}", self.suffix)
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::{StyleCompileContext, StyleScope};

	#[rstest]
	fn scope_identity_and_sha256_suffix_match_the_locked_vector() {
		// Arrange
		let context = StyleCompileContext {
			package_name: "poll-app",
			package_version: "0.4.0",
			style_type_name: "PollCardStyles",
		};

		// Act
		let scope = StyleScope::new(&context);

		// Assert
		assert_eq!(
			scope.identity,
			concat!("rstyle-v2\0poll-app\0", "0.4.0\0PollCardStyles")
		);
		assert_eq!(scope.suffix, "f69b9cbc74c9");
	}

	#[rstest]
	fn scope_identity_uses_the_generated_style_type_contract() {
		// Arrange
		let context = StyleCompileContext {
			package_name: "poll-app",
			package_version: "0.4.0",
			style_type_name: "CardStyles",
		};

		// Act
		let scope = StyleScope::new(&context);

		// Assert
		assert!(scope.identity.starts_with("rstyle-v2\0"));
	}

	#[rstest]
	fn generated_names_use_the_locked_scope_suffix() {
		// Arrange
		let context = StyleCompileContext {
			package_name: "poll-app",
			package_version: "0.4.0",
			style_type_name: "PollCardStyles",
		};
		let scope = StyleScope::new(&context);

		// Act
		let class_name = scope.class_name("poll-card");
		let variable_name = scope.variable_name("surface-secondary");

		// Assert
		assert_eq!(class_name, "poll-card--rs-f69b9cbc74c9");
		assert_eq!(variable_name, "--rs-f69b9cbc74c9-surface-secondary");
	}

	#[rstest]
	fn only_package_version_and_style_type_contribute_to_scope_identity() {
		// Arrange
		let base = StyleCompileContext {
			package_name: "poll-app",
			package_version: "0.4.0",
			style_type_name: "PollCardStyles",
		};
		let package_changed = StyleCompileContext {
			package_name: "other-app",
			..base
		};
		let version_changed = StyleCompileContext {
			package_version: "0.4.1",
			..base
		};
		let type_changed = StyleCompileContext {
			style_type_name: "OtherStyles",
			..base
		};

		// Act
		let base_scope = StyleScope::new(&base);
		let same_scope_after_unrelated_source_changes = StyleScope::new(&base);
		let changed_suffixes = [
			StyleScope::new(&package_changed).suffix,
			StyleScope::new(&version_changed).suffix,
			StyleScope::new(&type_changed).suffix,
		];

		// Assert
		assert_eq!(
			base_scope.suffix,
			same_scope_after_unrelated_source_changes.suffix
		);
		assert!(
			changed_suffixes
				.iter()
				.all(|suffix| suffix != &base_scope.suffix)
		);
	}
}
