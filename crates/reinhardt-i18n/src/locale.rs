//! Locale management functions
//!
//! These functions provide backward-compatible locale management.
//! For new code, prefer using `TranslationContext` with `set_active_translation()`.

use crate::{
	I18nError, MessageCatalog, TranslationContext, get_active_translation, set_active_translation,
};
use std::sync::Arc;

/// Validate locale string format
fn validate_locale(locale: &str) -> Result<(), I18nError> {
	// Basic validation: locale should contain only alphanumeric characters, hyphens, and underscores
	if locale.is_empty() {
		return Err(I18nError::InvalidLocale(
			"Locale cannot be empty".to_string(),
		));
	}

	if !locale
		.chars()
		.all(|c| c.is_alphanumeric() || c == '-' || c == '_')
	{
		return Err(I18nError::InvalidLocale(locale.to_string()));
	}

	Ok(())
}

/// Activate a locale by creating a new translation context.
///
/// **Note**: This function creates a new context with only the specified locale.
/// For full control, use `TranslationContext` with `set_active_translation()`.
///
/// **Warning**: The returned guard must be kept in scope for translations to work.
/// This is a change from the previous global state behavior.
///
/// # Example
///
/// ```
/// use reinhardt_i18n::{TranslationContext, set_active_translation, gettext, MessageCatalog};
/// use std::sync::Arc;
///
/// // Preferred approach: use TranslationContext directly
/// let mut ctx = TranslationContext::new("es", "en-US");
/// let mut catalog = MessageCatalog::new("es");
/// catalog.add_translation("Welcome", "Bienvenido");
/// ctx.add_catalog("es", catalog);
///
/// let _guard = set_active_translation(Arc::new(ctx));
/// assert_eq!(gettext("Welcome"), "Bienvenido");
/// ```
pub fn activate(locale: &str) -> Result<(), I18nError> {
	validate_locale(locale)?;

	// Get current context or create new one
	let mut ctx = get_active_translation()
		.map(|arc| (*arc).clone())
		.unwrap_or_else(|| TranslationContext::new("en-US", "en-US"));

	ctx.set_locale(locale);

	// Set the new context (this leaks the guard, but maintains backward compatibility)
	// In new code, users should use set_active_translation() directly
	let guard = set_active_translation(Arc::new(ctx));
	std::mem::forget(guard);

	Ok(())
}

/// Activate a locale with its message catalog directly
///
/// This creates a new translation context with the given locale and catalog.
///
/// **Warning**: The returned guard must be kept in scope for translations to work.
/// This is a change from the previous global state behavior.
///
/// # Example
///
/// ```
/// use reinhardt_i18n::{TranslationContext, set_active_translation, gettext, MessageCatalog};
/// use std::sync::Arc;
///
/// // Preferred approach: use TranslationContext directly
/// let mut ctx = TranslationContext::new("es", "en-US");
/// let mut catalog = MessageCatalog::new("es");
/// catalog.add_translation("Welcome", "Bienvenido");
/// ctx.add_catalog("es", catalog);
///
/// let _guard = set_active_translation(Arc::new(ctx));
/// assert_eq!(gettext("Welcome"), "Bienvenido");
/// ```
pub fn activate_with_catalog(locale: &str, catalog: MessageCatalog) {
	// Get current context or create new one
	let mut ctx = get_active_translation()
		.map(|arc| (*arc).clone())
		.unwrap_or_else(|| TranslationContext::new("en-US", "en-US"));

	ctx.set_locale(locale);
	ctx.add_catalog(locale, catalog);

	// Set the new context (this leaks the guard, but maintains backward compatibility)
	let guard = set_active_translation(Arc::new(ctx));
	std::mem::forget(guard);
}

/// Deactivate the current locale and revert to English
///
/// This sets the current locale to English (en-US).
///
/// # Example
///
/// ```
/// use reinhardt_i18n::{TranslationContext, set_active_translation, deactivate, gettext, MessageCatalog};
/// use std::sync::Arc;
///
/// let mut ctx = TranslationContext::new("de", "en-US");
/// let mut catalog = MessageCatalog::new("de");
/// catalog.add_translation("Hello", "Hallo");
/// ctx.add_catalog("de", catalog);
///
/// let _guard = set_active_translation(Arc::new(ctx));
/// assert_eq!(gettext("Hello"), "Hallo");
///
/// deactivate();
/// assert_eq!(gettext("Hello"), "Hello");
/// ```
pub fn deactivate() {
	// Get current context and reset locale to empty
	if let Some(arc) = get_active_translation() {
		let mut ctx = (*arc).clone();
		ctx.set_locale("");

		// Set the new context (this leaks the guard)
		let guard = set_active_translation(Arc::new(ctx));
		std::mem::forget(guard);
	}
}

/// Get the currently active locale
///
/// Returns "en-US" if no translation context is active.
///
/// # Example
///
/// ```
/// use reinhardt_i18n::{TranslationContext, set_active_translation, get_locale, MessageCatalog};
/// use std::sync::Arc;
///
/// // No active context
/// assert_eq!(get_locale(), "en-US");
///
/// let ctx = TranslationContext::new("it", "en-US");
/// let _guard = set_active_translation(Arc::new(ctx));
/// assert_eq!(get_locale(), "it");
/// ```
pub fn get_locale() -> String {
	get_active_translation()
		.map(|ctx| ctx.get_locale().to_string())
		.unwrap_or_else(|| "en-US".to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serial_test::serial;

	#[rstest]
	#[serial(i18n)]
	fn test_locale_activation() {
		let mut ctx = TranslationContext::new("pt", "en-US");
		let catalog = MessageCatalog::new("pt");
		ctx.add_catalog("pt", catalog);

		let _guard = set_active_translation(Arc::new(ctx));
		assert_eq!(get_locale(), "pt");
	}

	#[rstest]
	#[serial(i18n)]
	fn test_deactivate() {
		let mut ctx = TranslationContext::new("fr", "en-US");
		let catalog = MessageCatalog::new("fr");
		ctx.add_catalog("fr", catalog);

		let _guard = set_active_translation(Arc::new(ctx));
		assert_eq!(get_locale(), "fr");

		deactivate();
		assert_eq!(get_locale(), "en-US");
	}
}
