//! DI registrations for the snippets app.
//!
//! Provider functions return `FactoryOutput<K, T>`, so the `K` type is part of
//! the registry key. This lets one value type have multiple independently
//! registered providers without wrapping the value itself.

use reinhardt::di::{Depends, FactoryOutput, injectable, injectable_key};

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

#[injectable_key]
pub struct SnippetListConfigKey;

#[injectable(scope = "singleton")]
async fn snippet_list_config() -> FactoryOutput<SnippetListConfigKey, SnippetListConfig> {
	FactoryOutput::new(SnippetListConfig::default())
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
	#[inject] base: Depends<SnippetListConfigKey, SnippetListConfig>,
) -> FactoryOutput<CheckedSnippetListConfigKey, Result<SnippetListConfig, ConfigError>> {
	if base.max_page_size == 0 {
		return FactoryOutput::new(Err(ConfigError("max_page_size must be positive".into())));
	}
	FactoryOutput::new(Ok(SnippetListConfig {
		max_page_size: base.max_page_size,
	}))
}
