//! DI registrations for the snippets app.
//!
//! The DI registry keys every factory by the `TypeId` of its literal
//! return type. Two factories returning the same `T` would collide, so
//! when a second flavour of a dependency is needed, either introduce a
//! dedicated newtype or return `Result<T, FactoryLocalError>` — each
//! distinct error type yields a distinct registry key.
//!
//! [`SnippetListConfig`] is registered as a plain `#[injectable]` singleton
//! (key: `TypeId::of::<SnippetListConfig>()`). `checked_list_config` is a
//! second factory that also produces a `SnippetListConfig` on success, but
//! its return type is `Result<SnippetListConfig, ConfigError>`
//! (key: `TypeId::of::<Result<SnippetListConfig, ConfigError>>()`) — a
//! distinct registry entry that does not collide with the plain factory
//! above. `views::config` consumes it via `Depends<Result<SnippetListConfig,
//! ConfigError>>`.

use reinhardt::di::{Depends, injectable, injectable_factory};

/// Snippet listing configuration resolved through DI.
///
/// Registered as a singleton with no `#[inject]` fields, so
/// `#[injectable]` falls back to `Self::default()` — see the manual
/// `Default` impl below for the actual default value.
#[injectable(scope = "singleton")]
pub struct SnippetListConfig {
	#[no_inject]
	pub max_page_size: usize,
}

impl Default for SnippetListConfig {
	fn default() -> Self {
		// A non-zero default so the checked factory below succeeds out of the box.
		Self { max_page_size: 50 }
	}
}

/// Error type local to `checked_list_config`.
///
/// Its only job is to make the factory's `Result<SnippetListConfig,
/// ConfigError>` return type a distinct DI registry key from the plain
/// `SnippetListConfig` registered above.
#[derive(Debug)]
pub struct ConfigError(pub String);

/// Fallible variant of [`SnippetListConfig`], registered under the
/// `Result<SnippetListConfig, ConfigError>` key.
///
/// Re-validates the plain `SnippetListConfig` singleton and surfaces a
/// [`ConfigError`] if `max_page_size` is not positive, demonstrating how
/// `#[injectable_factory]` factories can return `Result<T, E>` to obtain a
/// registry key distinct from `T` while reusing `T`'s public fields.
#[injectable_factory(scope = "singleton")]
async fn checked_list_config(
	#[inject] base: Depends<SnippetListConfig>,
) -> Result<SnippetListConfig, ConfigError> {
	if base.max_page_size == 0 {
		return Err(ConfigError("max_page_size must be positive".into()));
	}
	Ok(SnippetListConfig {
		max_page_size: base.max_page_size,
	})
}
