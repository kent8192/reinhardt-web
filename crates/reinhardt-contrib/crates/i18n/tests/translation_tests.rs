//! Translation function tests
//!
//! Tests based on Django's i18n/tests.py - TranslationTests class

use reinhardt_i18n::{
    MessageCatalog, activate, deactivate, get_language, gettext, load_catalog, ngettext, npgettext,
    pgettext,
};
use serial_test::serial;
use std::sync::Mutex;

// Global lock to serialize tests that modify global state
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn setup_test_catalogs() {
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
}

#[test]
#[serial(i18n)]
fn test_plural_french() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

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

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_plural_null() {
    let _lock = TEST_LOCK.lock().unwrap();

    // When no translation is available, use default English rules
    deactivate();

    let result = ngettext("%(num)d year", "%(num)d years", 0);
    assert_eq!(result.replace("%(num)d", "0"), "0 years");

    let result = ngettext("%(num)d year", "%(num)d years", 1);
    assert_eq!(result.replace("%(num)d", "1"), "1 year");

    let result = ngettext("%(num)d year", "%(num)d years", 2);
    assert_eq!(result.replace("%(num)d", "2"), "2 years");
}

#[test]
#[serial(i18n)]
fn test_gettext_simple() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    activate("fr-FR").unwrap();

    let result = gettext("Hello");
    assert_eq!(result, "Bonjour");

    let result = gettext("Goodbye");
    assert_eq!(result, "Au revoir");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_gettext_untranslated() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();

    let result = gettext("Untranslated message");
    assert_eq!(result, "Untranslated message");
}

#[test]
#[serial(i18n)]
fn test_pgettext() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

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

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_npgettext() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    activate("de-DE").unwrap();

    let result = npgettext("search", "%(num)d result", "%(num)d results", 4);
    assert_eq!(result.replace("%(num)d", "4"), "4 Resultate");

    let result = npgettext("search", "%(num)d result", "%(num)d results", 1);
    assert_eq!(result.replace("%(num)d", "1"), "1 Resultat");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_empty_value() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    activate("de-DE").unwrap();

    // Empty value must stay empty after being translated
    let result = gettext("");
    assert_eq!(result, "");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_activate_deactivate() {
    let _lock = TEST_LOCK.lock().unwrap();

    // Initially should be fallback locale
    deactivate();
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

#[test]
#[serial(i18n)]
fn test_override_behavior() {
    let _lock = TEST_LOCK.lock().unwrap();

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

#[test]
#[serial(i18n)]
fn test_translation_invalid_locale() {
    let _lock = TEST_LOCK.lock().unwrap();

    // Invalid locale should return error
    let result = activate("123-@#$-invalid");
    assert!(result.is_err());
}

#[test]
#[serial(i18n)]
fn test_translation_ngettext_defaults() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();

    // Test default English plural rules
    let result_singular = ngettext("There is {} item", "There are {} items", 1);
    assert_eq!(result_singular, "There is {} item");

    let result_plural = ngettext("There is {} item", "There are {} items", 0);
    assert_eq!(result_plural, "There are {} items");

    let result_plural = ngettext("There is {} item", "There are {} items", 5);
    assert_eq!(result_plural, "There are {} items");
}

#[test]
#[serial(i18n)]
fn test_fallback_to_english() {
    let _lock = TEST_LOCK.lock().unwrap();

    // Activate a locale without catalog
    activate("ja-JP").unwrap();

    // Should fallback to untranslated message
    let result = gettext("Untranslated");
    assert_eq!(result, "Untranslated");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_get_language() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();
    assert_eq!(get_language(), "en-US");

    activate("fr-FR").unwrap();
    assert_eq!(get_language(), "fr-FR");

    activate("de-DE").unwrap();
    assert_eq!(get_language(), "de-DE");

    deactivate();
    assert_eq!(get_language(), "en-US");
}
