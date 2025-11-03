//! Translation function tests
//!
//! Tests based on Django's i18n/tests.py - TranslationTests class

use reinhardt_i18n::{
	MessageCatalog, activate, deactivate, get_language, gettext, load_catalog, ngettext, npgettext,
	pgettext,
};
use reinhardt_test::resource::{TeardownGuard, TestResource};
use rstest::*;
use serial_test::serial;

/// Test environment for i18n tests with automatic setup/teardown
struct I18nTestEnv;

impl TestResource for I18nTestEnv {
	fn setup() -> Self {
		// Setup French catalog
		let mut fr_catalog = MessageCatalog::new("fr-FR");

		// Add simple translations
		fr_catalog.add("Hello".to_string(), "Bonjour".to_string());
		fr_catalog.add("Goodbye".to_string(), "Au revoir".to_string());

		// Add plural translations (French: 0 is singular, >1 is plural)
		fr_catalog.add_plural(
			"%(num)d year".to_string(),
			vec!["%(num)d année".to_string(), "%(num)d ans".to_string()],
		);
		fr_catalog.add_plural(
			"%(size)d byte".to_string(),
			vec!["%(size)d octet".to_string(), "%(size)d octets".to_string()],
		);

		// Add context translations
		fr_catalog.add_context(
			"month name".to_string(),
			"May".to_string(),
			"Mai".to_string(),
		);
		fr_catalog.add_context("verb".to_string(), "May".to_string(), "Kann".to_string());

		// Add contextual plural
		fr_catalog.add_plural(
			"search:%(num)d result".to_string(),
			vec![
				"%(num)d Resultat".to_string(),
				"%(num)d Resultate".to_string(),
			],
		);

		load_catalog("fr-FR", fr_catalog).unwrap();

		// Setup German catalog
		let mut de_catalog = MessageCatalog::new("de-DE");
		de_catalog.add("Password".to_string(), "Passwort".to_string());
		de_catalog.add_context(
			"month name".to_string(),
			"May".to_string(),
			"Mai".to_string(),
		);
		de_catalog.add_context("verb".to_string(), "May".to_string(), "Kann".to_string());
		de_catalog.add_plural(
			"search:%(num)d result".to_string(),
			vec![
				"%(num)d Resultat".to_string(),
				"%(num)d Resultate".to_string(),
			],
		);

		load_catalog("de-DE", de_catalog).unwrap();

		// Setup Polish catalog
		let mut pl_catalog = MessageCatalog::new("pl-PL");
		pl_catalog.add("Hello".to_string(), "Witaj".to_string());
		pl_catalog.add("Add %(name)s".to_string(), "Dodaj %(name)s".to_string());

		load_catalog("pl-PL", pl_catalog).unwrap();

		Self
	}

	fn teardown(&mut self) {
		// Deactivate any active language
		deactivate();
	}
}

/// Fixture providing i18n test environment with automatic cleanup
#[fixture]
fn i18n_ctx() -> TeardownGuard<I18nTestEnv> {
	TeardownGuard::new()
}

#[rstest]
#[serial(i18n)]
fn test_plural_french(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	activate("fr-FR").unwrap();

	// French: 0 is singular
	let result = ngettext("%(num)d year", "%(num)d years", 0);
	assert_eq!(result.replace("%(num)d", "0"), "0 année");

	// French: 2 is plural
	let result = ngettext("%(num)d year", "%(num)d years", 2);
	assert_eq!(result.replace("%(num)d", "2"), "2 ans");

	// Another plural test
	let result = ngettext("%(size)d byte", "%(size)d bytes", 0);
	assert_eq!(result.replace("%(size)d", "0"), "0 octet");

	let result = ngettext("%(size)d byte", "%(size)d bytes", 2);
	assert_eq!(result.replace("%(size)d", "2"), "2 octets");

	// teardown() is automatically called
}

#[rstest]
#[serial(i18n)]
fn test_plural_null(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// When no translation is available, use default English rules
	// (deactivate is called in teardown)

	let result = ngettext("%(num)d year", "%(num)d years", 0);
	assert_eq!(result.replace("%(num)d", "0"), "0 years");

	let result = ngettext("%(num)d year", "%(num)d years", 1);
	assert_eq!(result.replace("%(num)d", "1"), "1 year");

	let result = ngettext("%(num)d year", "%(num)d years", 2);
	assert_eq!(result.replace("%(num)d", "2"), "2 years");
}

#[rstest]
#[serial(i18n)]
fn test_gettext_simple(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	activate("fr-FR").unwrap();

	let result = gettext("Hello");
	assert_eq!(result, "Bonjour");

	let result = gettext("Goodbye");
	assert_eq!(result, "Au revoir");
}

#[rstest]
#[serial(i18n)]
fn test_gettext_untranslated(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// No activation - should use untranslated message

	let result = gettext("Untranslated message");
	assert_eq!(result, "Untranslated message");
}

#[rstest]
#[serial(i18n)]
fn test_pgettext(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	activate("de-DE").unwrap();

	// Unexisting context returns original message
	let result = pgettext("unexisting", "May");
	assert_eq!(result, "May");

	// Context "month name"
	let result = pgettext("month name", "May");
	assert_eq!(result, "Mai");

	// Context "verb"
	let result = pgettext("verb", "May");
	assert_eq!(result, "Kann");
}

#[rstest]
#[serial(i18n)]
fn test_npgettext(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	activate("de-DE").unwrap();

	let result = npgettext("search", "%(num)d result", "%(num)d results", 4);
	assert_eq!(result.replace("%(num)d", "4"), "4 Resultate");

	let result = npgettext("search", "%(num)d result", "%(num)d results", 1);
	assert_eq!(result.replace("%(num)d", "1"), "1 Resultat");
}

#[rstest]
#[serial(i18n)]
fn test_empty_value(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	activate("de-DE").unwrap();

	// Empty value must stay empty after being translated
	let result = gettext("");
	assert_eq!(result, "");
}

#[rstest]
#[serial(i18n)]
fn test_activate_deactivate(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// Initially should be fallback locale (teardown calls deactivate)
	assert_eq!(get_language(), "en-US");

	// Activate German
	activate("de-DE").unwrap();
	assert_eq!(get_language(), "de-DE");

	// Activate French
	activate("fr-FR").unwrap();
	assert_eq!(get_language(), "fr-FR");

	// Deactivate - return to fallback
	deactivate();
	assert_eq!(get_language(), "en-US");
}

#[rstest]
#[serial(i18n)]
fn test_override_behavior(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// Activate German
	activate("de-DE").unwrap();
	assert_eq!(get_language(), "de-DE");

	// Override with Polish
	activate("pl-PL").unwrap();
	assert_eq!(get_language(), "pl-PL");

	// Go back to German
	activate("de-DE").unwrap();
	assert_eq!(get_language(), "de-DE");

	// Deactivate
	deactivate();
	assert_eq!(get_language(), "en-US");
}

#[rstest]
#[serial(i18n)]
fn test_translation_invalid_locale(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// Invalid locale should return error
	let result = activate("123-@#$-invalid");
	assert!(result.is_err());
}

#[rstest]
#[serial(i18n)]
fn test_translation_ngettext_defaults(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// Use default English rules (no activation)

	// Test default English plural rules
	let result_singular = ngettext("There is {} item", "There are {} items", 1);
	assert_eq!(result_singular, "There is {} item");

	let result_plural = ngettext("There is {} item", "There are {} items", 0);
	assert_eq!(result_plural, "There are {} items");

	let result_plural = ngettext("There is {} item", "There are {} items", 5);
	assert_eq!(result_plural, "There are {} items");
}

#[rstest]
#[serial(i18n)]
fn test_fallback_to_english(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	// Activate a locale without catalog
	activate("ja-JP").unwrap();

	// Should fallback to untranslated message
	let result = gettext("Untranslated");
	assert_eq!(result, "Untranslated");
}

#[rstest]
#[serial(i18n)]
fn test_get_language(_i18n_ctx: TeardownGuard<I18nTestEnv>) {
	assert_eq!(get_language(), "en-US");

	activate("fr-FR").unwrap();
	assert_eq!(get_language(), "fr-FR");

	activate("de-DE").unwrap();
	assert_eq!(get_language(), "de-DE");

	deactivate();
	assert_eq!(get_language(), "en-US");
}
