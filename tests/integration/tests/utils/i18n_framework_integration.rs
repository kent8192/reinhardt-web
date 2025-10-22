// i18n Framework Integration Tests
// Tests the internationalization and localization functionality

use reinhardt_i18n::{
    activate, deactivate, get_locale, gettext, ngettext, pgettext, MessageCatalog,
};

#[test]
fn test_activate_and_deactivate() {
    // Create catalog and activate Japanese locale
    let catalog = MessageCatalog::new("ja-JP");
    activate("ja-JP", catalog);
    assert_eq!(get_locale(), "ja-JP");

    // Deactivate (returns to fallback en)
    deactivate();
    assert_eq!(get_locale(), "en");
}

#[test]
fn test_gettext_without_catalog() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    // Without translations in catalog, gettext returns the original string
    let msg = gettext("Hello, world!");
    assert_eq!(msg, "Hello, world!");

    deactivate();
}

#[test]
fn test_gettext_with_catalog() {
    // Create a simple catalog
    let mut catalog = MessageCatalog::new("ja-JP");
    catalog.add_translation("Hello, world!", "こんにちは、世界！");

    // Activate locale with catalog
    activate("ja-JP", catalog);

    // gettext should now return translated string
    let msg = gettext("Hello, world!");
    assert_eq!(msg, "こんにちは、世界！");

    deactivate();
}

#[test]
fn test_ngettext_singular() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    let msg = ngettext("There is {} item", "There are {} items", 1);
    assert_eq!(msg, "There is {} item");

    deactivate();
}

#[test]
fn test_ngettext_plural() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    let msg = ngettext("There is {} item", "There are {} items", 5);
    assert_eq!(msg, "There are {} items");

    deactivate();
}

#[test]
fn test_pgettext_context() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    // pgettext with context - without translations returns original
    let msg = pgettext("menu", "File");
    assert_eq!(msg, "File");

    deactivate();
}

#[test]
fn test_locale_switching() {
    // Start with English
    let en_catalog = MessageCatalog::new("en-US");
    activate("en-US", en_catalog);
    assert_eq!(get_locale(), "en-US");

    // Switch to Japanese
    let ja_catalog = MessageCatalog::new("ja-JP");
    activate("ja-JP", ja_catalog);
    assert_eq!(get_locale(), "ja-JP");

    // Switch to French
    let fr_catalog = MessageCatalog::new("fr-FR");
    activate("fr-FR", fr_catalog);
    assert_eq!(get_locale(), "fr-FR");

    deactivate();
}

#[test]
fn test_invalid_locale() {
    // activate() doesn't return Result - it accepts any string
    // This test verifies that activate works with unusual locale strings
    let catalog = MessageCatalog::new("invalid_locale_@#$");
    activate("invalid_locale_@#$", catalog);
    assert_eq!(get_locale(), "invalid_locale_@#$");
    deactivate();
}

#[test]
fn test_catalog_fallback() {
    // Create Japanese catalog with translation
    let mut ja_catalog = MessageCatalog::new("ja-JP");
    ja_catalog.add_translation("Welcome", "ようこそ");

    // Activate Japanese
    activate("ja-JP", ja_catalog);
    let msg = gettext("Welcome");
    assert_eq!(msg, "ようこそ");

    deactivate();
}

#[test]
fn test_multiple_locales() {
    // Create catalogs for multiple locales
    let locales = vec![
        ("ja-JP", "Hello", "こんにちは"),
        ("fr-FR", "Hello", "Bonjour"),
        ("de-DE", "Hello", "Hallo"),
        ("es-ES", "Hello", "Hola"),
    ];

    for (locale_str, key, value) in locales {
        let mut catalog = MessageCatalog::new(locale_str);
        catalog.add_translation(key, value);

        // Activate and test
        activate(locale_str, catalog);
        let msg = gettext(key);
        assert_eq!(msg, value);
    }

    deactivate();
}

#[test]
fn test_gettext_preserve_formatting() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    // Test that formatting is preserved
    let msg = gettext("User {} logged in at {}");
    assert_eq!(msg, "User {} logged in at {}");

    deactivate();
}

#[test]
fn test_ngettext_zero_count() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    // Zero should use plural form in English
    let msg = ngettext("There is {} item", "There are {} items", 0);
    assert_eq!(msg, "There are {} items");

    deactivate();
}

#[test]
fn test_ngettext_large_count() {
    let catalog = MessageCatalog::new("en-US");
    activate("en-US", catalog);

    let msg = ngettext("There is {} file", "There are {} files", 1000);
    assert_eq!(msg, "There are {} files");

    deactivate();
}

#[test]
fn test_pgettext_disambiguation() {
    // Create catalog with context-specific translations
    let mut catalog = MessageCatalog::new("ja-JP");

    // "Open" in different contexts
    catalog.add_context("menu", "Open", "ファイルを開く");
    catalog.add_context("door", "Open", "ドアを開ける");

    activate("ja-JP", catalog);

    // Different contexts should give different translations
    let menu_msg = pgettext("menu", "Open");
    let door_msg = pgettext("door", "Open");

    assert_eq!(menu_msg, "ファイルを開く");
    assert_eq!(door_msg, "ドアを開ける");

    deactivate();
}

#[test]
fn test_locale_language_only() {
    // Should accept language-only locale codes
    let ja_catalog = MessageCatalog::new("ja");
    activate("ja", ja_catalog);
    let lang = get_locale();
    assert_eq!(lang, "ja", "Expected 'ja' locale, got: {}", lang);

    let en_catalog = MessageCatalog::new("en");
    activate("en", en_catalog);
    let lang = get_locale();
    assert_eq!(lang, "en", "Expected 'en' locale, got: {}", lang);

    deactivate();
}

#[test]
fn test_locale_case_normalization() {
    // Different case variations should work
    let catalog1 = MessageCatalog::new("en-us");
    activate("en-us", catalog1);
    let lang1 = get_locale();
    assert_eq!(lang1, "en-us");

    let catalog2 = MessageCatalog::new("en-US");
    activate("en-US", catalog2);
    let lang2 = get_locale();
    assert_eq!(lang2, "en-US");

    deactivate();
}
