//! Lazy translation tests
//!
//! Tests based on Django's i18n/tests.py - lazy translation functionality

use reinhardt_i18n::{
    MessageCatalog, activate, deactivate, gettext_lazy, load_catalog, ngettext_lazy,
};
use serial_test::serial;
use std::sync::Mutex;

// Global lock to serialize tests that modify global state
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn setup_test_catalogs() {
    // Setup French catalog
    let mut fr_catalog = MessageCatalog::new("fr-FR");
    fr_catalog.add("Add %(name)s".to_string(), "Ajouter %(name)s".to_string());
    fr_catalog.add("Hello".to_string(), "Bonjour".to_string());
    fr_catalog.add_plural(
        "%(count)d item".to_string(),
        vec![
            "%(count)d élément".to_string(),
            "%(count)d éléments".to_string(),
        ],
    );

    load_catalog("fr-FR", fr_catalog).unwrap();

    // Setup German catalog
    let mut de_catalog = MessageCatalog::new("de-DE");
    de_catalog.add(
        "Add %(name)s".to_string(),
        "%(name)s hinzufügen".to_string(),
    );
    de_catalog.add("Hello".to_string(), "Hallo".to_string());
    de_catalog.add_plural(
        "%(count)d item".to_string(),
        vec![
            "%(count)d Artikel".to_string(),
            "%(count)d Artikel".to_string(),
        ],
    );

    load_catalog("de-DE", de_catalog).unwrap();

    // Setup Polish catalog
    let mut pl_catalog = MessageCatalog::new("pl-PL");
    pl_catalog.add("Add %(name)s".to_string(), "Dodaj %(name)s".to_string());
    pl_catalog.add("Hello".to_string(), "Witaj".to_string());

    load_catalog("pl-PL", pl_catalog).unwrap();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_basic() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = gettext_lazy("Hello");

    // Initially in English
    deactivate();
    assert_eq!(lazy_msg.to_string(), "Hello");

    // Activate French
    activate("fr-FR").unwrap();
    assert_eq!(lazy_msg.to_string(), "Bonjour");

    // Activate German
    activate("de-DE").unwrap();
    assert_eq!(lazy_msg.to_string(), "Hallo");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_translation_string_display() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = gettext_lazy("Hello");

    activate("fr-FR").unwrap();
    let displayed = format!("{}", lazy_msg);
    assert_eq!(displayed, "Bonjour");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_interpolation() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = gettext_lazy("Add %(name)s");

    // Test in French
    activate("fr-FR").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(name)s", "Ringo");
    assert_eq!(result, "Ajouter Ringo");

    // Test in German
    activate("de-DE").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(name)s", "Ringo");
    assert_eq!(result, "Ringo hinzufügen");

    // Test in Polish
    activate("pl-PL").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(name)s", "Ringo");
    assert_eq!(result, "Dodaj Ringo");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_ngettext_lazy_basic() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = ngettext_lazy("%(count)d item", "%(count)d items", 1);

    deactivate();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "1");
    assert_eq!(result, "1 item");

    activate("fr-FR").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "1");
    assert_eq!(result, "1 élément");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_ngettext_lazy_plural() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = ngettext_lazy("%(count)d item", "%(count)d items", 5);

    deactivate();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "5");
    assert_eq!(result, "5 items");

    activate("fr-FR").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "5");
    assert_eq!(result, "5 éléments");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_clone() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg1 = gettext_lazy("Hello");
    let lazy_msg2 = lazy_msg1.clone();

    activate("fr-FR").unwrap();
    assert_eq!(lazy_msg1.to_string(), "Bonjour");
    assert_eq!(lazy_msg2.to_string(), "Bonjour");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_untranslated() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();

    let lazy_msg = gettext_lazy("Untranslated message");
    assert_eq!(lazy_msg.to_string(), "Untranslated message");

    activate("fr-FR").unwrap();
    assert_eq!(lazy_msg.to_string(), "Untranslated message");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_empty() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();

    let lazy_msg = gettext_lazy("");
    assert_eq!(lazy_msg.to_string(), "");

    activate("fr-FR").unwrap();
    assert_eq!(lazy_msg.to_string(), "");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_evaluation_timing() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    // Create lazy string before activating locale
    deactivate();
    let lazy_msg = gettext_lazy("Hello");

    // Lazy string should not be evaluated yet
    // Activate French and then evaluate
    activate("fr-FR").unwrap();
    assert_eq!(lazy_msg.to_string(), "Bonjour");

    // Change locale and re-evaluate
    activate("de-DE").unwrap();
    assert_eq!(lazy_msg.to_string(), "Hallo");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_ngettext_lazy_zero() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy_msg = ngettext_lazy("%(count)d item", "%(count)d items", 0);

    deactivate();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "0");
    assert_eq!(result, "0 items");

    activate("fr-FR").unwrap();
    let msg = lazy_msg.to_string();
    let result = msg.replace("%(count)d", "0");
    // French: 0 is singular
    assert_eq!(result, "0 élément");

    deactivate();
}

#[test]
#[serial(i18n)]
fn test_lazy_string_debug() {
    let _lock = TEST_LOCK.lock().unwrap();

    deactivate();

    let lazy_msg = gettext_lazy("Test message");
    let debug_str = format!("{:?}", lazy_msg);

    // Debug representation should contain the message
    assert!(debug_str.contains("Test message") || debug_str.contains("LazyString"));
}

#[test]
#[serial(i18n)]
fn test_multiple_lazy_strings() {
    let _lock = TEST_LOCK.lock().unwrap();
    setup_test_catalogs();

    let lazy1 = gettext_lazy("Hello");
    let lazy2 = gettext_lazy("Add %(name)s");

    activate("fr-FR").unwrap();
    assert_eq!(lazy1.to_string(), "Bonjour");
    assert_eq!(
        lazy2.to_string().replace("%(name)s", "Test"),
        "Ajouter Test"
    );

    activate("de-DE").unwrap();
    assert_eq!(lazy1.to_string(), "Hallo");
    assert_eq!(
        lazy2.to_string().replace("%(name)s", "Test"),
        "Test hinzufügen"
    );

    deactivate();
}
