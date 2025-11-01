//! Locale management functions

use crate::{I18nError, MessageCatalog, TRANSLATION_STATE};

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

/// Activate a locale (loads catalog if already registered via load_catalog)
///
/// # Example
/// ```
/// use reinhardt_i18n::{activate, load_catalog, gettext, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("es");
/// catalog.add_translation("Welcome", "Bienvenido");
/// load_catalog("es", catalog).unwrap();
///
/// activate("es").unwrap();
///
/// assert_eq!(gettext("Welcome"), "Bienvenido");
/// ```
pub fn activate(locale: &str) -> Result<(), I18nError> {
	validate_locale(locale)?;

	// Always allow activation - catalogs can be loaded separately via load_catalog
	// This matches Django's behavior where activate() can be called before loading catalogs
	let mut state = TRANSLATION_STATE.write().unwrap();
	state.set_locale(locale.to_string());
	Ok(())
}

/// Activate a locale with its message catalog directly
///
/// This is the low-level API that combines catalog loading and activation.
///
/// # Example
/// ```
/// use reinhardt_i18n::{activate_with_catalog, gettext, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("es");
/// catalog.add_translation("Welcome", "Bienvenido");
///
/// activate_with_catalog("es", catalog);
///
/// assert_eq!(gettext("Welcome"), "Bienvenido");
/// ```
pub fn activate_with_catalog(locale: &str, catalog: MessageCatalog) {
	let mut state = TRANSLATION_STATE.write().unwrap();
	state.set_locale(locale.to_string());
	state.add_catalog(locale.to_string(), catalog);
}

/// Deactivate the current locale and revert to English
///
/// # Example
/// ```
/// use reinhardt_i18n::{activate_with_catalog, deactivate, gettext, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("de");
/// catalog.add_translation("Hello", "Hallo");
///
/// activate_with_catalog("de", catalog);
/// assert_eq!(gettext("Hello"), "Hallo");
///
/// deactivate();
/// assert_eq!(gettext("Hello"), "Hello");
/// ```
pub fn deactivate() {
	let mut state = TRANSLATION_STATE.write().unwrap();
	state.set_locale(String::new()); // Empty string will return "en-US" via get_locale()
}

/// Get the currently active locale
///
/// # Example
/// ```
/// use reinhardt_i18n::{activate_with_catalog, get_locale, MessageCatalog};
///
/// assert_eq!(get_locale(), "en-US");
///
/// let catalog = MessageCatalog::new("it");
/// activate_with_catalog("it", catalog);
///
/// assert_eq!(get_locale(), "it");
/// ```
pub fn get_locale() -> String {
	let state = TRANSLATION_STATE.read().unwrap();
	state.get_locale().to_string()
}

#[cfg(test)]
mod tests {
	use super::*;
	use serial_test::serial;

	#[test]
	#[serial(i18n)]
	fn test_locale_activation() {
		let catalog = MessageCatalog::new("pt");
		activate_with_catalog("pt", catalog);
		assert_eq!(get_locale(), "pt");

		deactivate();
		assert_eq!(get_locale(), "en-US");
	}
}
