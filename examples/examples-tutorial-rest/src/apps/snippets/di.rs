//! DI registrations for the snippets app.
//!
//! Provider functions can return the dependency value directly for the common
//! one-provider-per-type case. Use `KeyedFactoryOutput<K, T>` only when a value
//! type needs multiple independently registered providers.

use reinhardt::di::{Depends, KeyedFactoryOutput, injectable, injectable_key};

/// Snippet listing configuration resolved through DI.
pub struct SnippetListConfig {
	pub max_page_size: usize,
}

impl Default for SnippetListConfig {
	fn default() -> Self {
		// A non-zero default so the checked factory below succeeds out of the box.
		Self { max_page_size: 50 }
	}
}

#[injectable(scope = "singleton")]
async fn snippet_list_config() -> SnippetListConfig {
	SnippetListConfig::default()
}

/// Error type local to `checked_list_config`.
#[derive(Debug)]
pub struct ConfigError(pub String);

#[injectable_key]
pub struct CheckedSnippetListConfigKey;

/// Fallible variant of [`SnippetListConfig`], registered under the
/// [`CheckedSnippetListConfigKey`] key.
///
/// Re-validates the plain `SnippetListConfig` singleton and surfaces a
/// [`ConfigError`] if `max_page_size` is not positive, demonstrating how
/// provider functions can return `Result<T, E>` without using the error type
/// as an ad hoc registry key.
#[injectable(scope = "singleton")]
async fn checked_list_config(
	#[inject] base: Depends<SnippetListConfig>,
) -> KeyedFactoryOutput<CheckedSnippetListConfigKey, Result<SnippetListConfig, ConfigError>> {
	if base.max_page_size == 0 {
		return KeyedFactoryOutput::new(Err(ConfigError("max_page_size must be positive".into())));
	}
	KeyedFactoryOutput::new(Ok(SnippetListConfig {
		max_page_size: base.max_page_size,
	}))
}
