//! Composable settings fragment trait
//!
//! Defines the [`SettingsFragment`] trait that root settings fragments implement.
//! Each root fragment maps to a TOML section and can be validated independently.
//!
//! # TOML section mapping
//!
//! Each fragment's [`SettingsFragment::section()`] determines which TOML section it
//! deserializes from. For example, `CoreSettings` (section `"core"`) reads from `[core]`,
//! and `I18nSettings` (section `"i18n"`) reads from `[i18n]`.
//!
//! # Nested structs within fragments
//!
//! When a fragment contains nested structs (e.g., `CoreSettings.security`), the
//! nested struct maps to a TOML sub-section named after the field:
//!
//! ```toml
//! [core]
//! secret_key = "..."
//! debug = false
//!
//! [core.security]
//! secure_ssl_redirect = true
//! session_cookie_secure = true
//! ```
//!
//! In the legacy `Settings` format (where `CoreSettings` is flattened at the root),
//! the sub-section becomes a top-level section:
//!
//! ```toml
//! secret_key = "..."
//! debug = false
//!
//! [security]
//! secure_ssl_redirect = true
//! session_cookie_secure = true
//! ```
//!
//! # Typed schema references
//!
//! Composed settings expose a typed schema through `ProjectSettings::schema()`.
//! Embedded settings nodes can be followed with field access, for example
//! `ProjectSettings::schema().database.default.password`. The rendered path is
//! composed from the root composition key, the embedded field key, and serde
//! rename attributes, such as `database.default.db-password`.
//!
//! Schema generation peels semantically agnostic wrappers when producing nested
//! references: `Option<T>`, `Vec<T>`, `HashMap<String, T>`,
//! `BTreeMap<String, T>`, `IndexMap<String, T>`, and `Box<T>`.
//! `#[setting(node)]` forces a nested settings node, while `#[setting(leaf)]`
//! keeps a field as a leaf. Without a shape hint, `*Config` types may infer node
//! behavior; `*Settings` types should be annotated explicitly unless they are
//! built-in fragments annotated by the crate.
//!
//! Recursive required-field validation reports missing nested required leaves as
//! [`BuildError::MissingRequiredPath`](super::builder::BuildError::MissingRequiredPath).
//! A missing direct field on the fragment section remains
//! [`BuildError::MissingRequiredField`](super::builder::BuildError::MissingRequiredField).
//!
//! A struct annotated with `#[settings(fragment = true)]` but no `section = "..."`
//! is an embedded settings node. It implements
//! [`SettingsNode`](super::schema::SettingsNode) for recursive schema support,
//! but not [`SettingsFragment`], because it is not a top-level composition
//! section.
//!
//! # Merge semantics
//!
//! When multiple TOML files are merged (e.g., `base.toml` + `local.toml`),
//! the resulting layout depends on the
//! [`MergeStrategy`](super::builder::MergeStrategy) chosen on the
//! [`SettingsBuilder`](super::builder::SettingsBuilder):
//!
//! - With [`MergeStrategy::Shallow`](super::builder::MergeStrategy::Shallow)
//!   — the legacy default for
//!   [`build`](super::builder::SettingsBuilder::build) — each top-level
//!   section is replaced as a whole. If `local.toml` defines `[core]`, it
//!   replaces the entire `[core]` section from `base.toml`, so
//!   environment-specific files must be self-contained for any section
//!   they touch.
//! - With [`MergeStrategy::Deep`](super::builder::MergeStrategy::Deep)
//!   — the default for
//!   [`build_composed`](super::builder::SettingsBuilder::build_composed)
//!   — nested tables are merged recursively. Defining `[core].debug =
//!   true` in `local.toml` no longer erases sibling fields like
//!   `[core].secret_key` or sub-sections like `[core.security]` that were
//!   set in `base.toml`. Arrays and scalars are still replaced
//!   wholesale.
//!
//! See [issue #4260](https://github.com/kent8192/reinhardt-web/issues/4260)
//! for the design discussion.

use super::policy::FieldPolicy;
use super::profile::Profile;
use super::validation::ValidationResult;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// Profile-aware validation for settings fragments.
///
/// Implement this trait to add custom validation logic to a settings fragment.
/// The `#[settings(fragment = true)]` macro generates a default (no-op) implementation
/// automatically. To provide custom validation, use `validate = false` in the macro
/// and implement this trait manually:
///
/// ```ignore
/// #[settings(fragment = true, section = "security", validate = false)]
/// struct SecuritySettings { /* ... */ }
///
/// impl SettingsValidation for SecuritySettings {
///     fn validate(&self, profile: &Profile) -> ValidationResult {
///         // custom validation logic
///         Ok(())
///     }
/// }
/// ```
pub trait SettingsValidation {
	/// Validate this fragment against the given profile.
	///
	/// Default implementation: no-op (always valid).
	fn validate(&self, _profile: &Profile) -> ValidationResult {
		Ok(())
	}
}

/// A rootable composable unit of configuration.
///
/// Each root fragment maps to a TOML section and can be validated independently.
/// Root fragments are composed into a `ProjectSettings` struct using the
/// `#[settings(key: Type | Type)]` macro.
///
/// # Implementing
///
/// Use `#[settings(fragment = true, section = "...")]` to auto-derive this trait,
/// or implement it manually for custom validation. Omit `section = "..."`
/// only for embedded settings nodes that should not be composed as root
/// fragments.
pub trait SettingsFragment:
	Clone + Debug + Serialize + DeserializeOwned + Send + Sync + 'static
{
	/// The accessor trait for this fragment.
	///
	/// Expresses the type-level association between a settings fragment
	/// and its `Has*Settings` accessor trait. Use `()` for fragments
	/// without a dedicated accessor trait.
	type Accessor: ?Sized;

	/// TOML section name (e.g., `"cache"`, `"core"`).
	fn section() -> &'static str;

	/// Validate this fragment against the given profile.
	///
	/// Default implementation: no-op (always valid).
	/// For custom validation, implement [`SettingsValidation`] and override
	/// this method to delegate to it.
	fn validate(&self, _profile: &Profile) -> ValidationResult {
		Ok(())
	}

	/// Returns the default field policies defined by the library author.
	///
	/// Generated by the `#[settings(fragment = true, section = "...")]` macro
	/// from `#[setting(...)]` field attributes. Returns empty slice by default
	/// for backward compatibility with existing fragments.
	fn field_policies() -> &'static [FieldPolicy] {
		&[]
	}
}

/// Generic accessor trait for settings fragments.
///
/// The `#[settings]` macro generates implementations of this trait
/// using fully-qualified paths, so users do not need to manually
/// import individual `Has*Settings` traits.
///
/// Fragment macros bridge `HasSettings<F>` to the specific
/// `Has*Settings` traits for each generated fragment.
pub trait HasSettings<F: SettingsFragment> {
	/// Returns a reference to the contained fragment.
	fn get_settings(&self) -> &F;
}

/// Marker trait bundling the commonly required fragment accessors used by
/// management commands and the database layer.
///
/// A composed settings type that derives `HasCoreSettings` and
/// `HasContactSettings` (i.e. it includes both `CoreSettings` and
/// `ContactSettings` fragments) automatically satisfies this trait via
/// the blanket implementation below.
///
/// Consumers that need an erased handle to settings (for example
/// `CommandContext`) hold an `Arc<dyn HasCommonSettings>` instead of a
/// concrete type, enabling cross-crate trait-object plumbing without
/// leaking the legacy `Settings` type.
pub trait HasCommonSettings:
	super::core_settings::HasCoreSettings + super::contacts::HasContactSettings + Send + Sync + 'static
{
}

impl<T> HasCommonSettings for T where
	T: super::core_settings::HasCoreSettings
		+ super::contacts::HasContactSettings
		+ Send
		+ Sync
		+ 'static
{
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::settings::profile::Profile;
	use rstest::rstest;

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	struct TestFragment {
		pub value: String,
	}

	impl SettingsFragment for TestFragment {
		type Accessor = ();

		fn section() -> &'static str {
			"test"
		}
	}

	#[rstest]
	fn test_settings_fragment_section() {
		// Arrange
		// (no setup needed)

		// Act
		let section = TestFragment::section();

		// Assert
		assert_eq!(section, "test");
	}

	#[rstest]
	fn test_settings_fragment_validate_default_ok() {
		// Arrange
		let fragment = TestFragment {
			value: "hello".to_string(),
		};
		let profile = Profile::Development;

		// Act
		let result = fragment.validate(&profile);

		// Assert
		assert!(result.is_ok());
	}
}
