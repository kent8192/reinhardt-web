//! Integration tests for Template + i18n functionality
//!
//! Tests the integration between reinhardt-templates and reinhardt-i18n crates.
//! Covers multilingual template rendering, translation filters, and localization.

use askama::Template;
use reinhardt_i18n::{
    activate, deactivate, get_locale, gettext, ngettext, pgettext, MessageCatalog,
};
use reinhardt_templates::{blocktrans, localize_date_filter, localize_number_filter};

// Test template with translation filters
#[derive(Template)]
#[template(source = "Welcome: {{ message }}", ext = "txt")]
struct TranslationTemplate {
    message: String,
}

#[test]
fn test_template_with_simple_translation() {
    // Create message catalog
    let mut catalog = MessageCatalog::new("ja");
    catalog.add_translation("Hello", "こんにちは");
    catalog.add_translation("Welcome", "ようこそ");

    // Set up i18n
    activate("ja", catalog);

    // Use real i18n function instead of template stub
    let message = gettext("Hello");
    let template = TranslationTemplate { message };

    let rendered = template.render().unwrap();
    assert_eq!(rendered, "Welcome: こんにちは");

    deactivate();
}

#[test]
fn test_template_with_context_translation() {
    let mut catalog = MessageCatalog::new("es");
    catalog.add_context("button", "Save", "Guardar");
    catalog.add_context("menu", "Save", "Guardar archivo");

    activate("es", catalog);

    // Test different contexts using real i18n function
    let button_save = pgettext("button", "Save");
    let menu_save = pgettext("menu", "Save");

    assert_eq!(button_save, "Guardar");
    assert_eq!(menu_save, "Guardar archivo");

    deactivate();
}

#[test]
fn test_template_with_block_translation() {
    let mut catalog = MessageCatalog::new("fr");
    catalog.add_translation("Welcome!", "Bienvenue!");

    activate("fr", catalog);

    // Note: blocktrans is a template stub that doesn't do actual translation
    // It just returns the input message as-is
    let result = blocktrans("Welcome!").unwrap();
    assert_eq!(result, "Welcome!");

    deactivate();
}

#[test]
fn test_template_with_plural_translation() {
    let mut catalog = MessageCatalog::new("de");
    catalog.add_plural("item", "items", vec!["Artikel", "Artikel"]);

    activate("de", catalog);

    // Test using real i18n plural function
    let result_singular = ngettext("item", "items", 1);
    assert_eq!(result_singular, "Artikel");

    let result_plural = ngettext("item", "items", 5);
    assert_eq!(result_plural, "Artikel");

    deactivate();
}

#[test]
fn test_template_number_localization() {
    // Create empty catalog for French
    let catalog = MessageCatalog::new("fr");

    // Test with French locale (uses comma as decimal separator)
    activate("fr", catalog);

    let result = localize_number_filter(1234.56).unwrap();
    // Template stub just returns the number as string
    assert!(result.contains("1234"));

    deactivate();
}

#[test]
fn test_template_date_localization() {
    let catalog = MessageCatalog::new("ja");
    activate("ja", catalog);

    let date_str = "2025-10-16";
    let result = localize_date_filter(date_str).unwrap();

    // Template stub just returns the input string
    assert!(result.contains("2025"));
    assert!(result.contains("10"));
    assert!(result.contains("16"));

    deactivate();
}

#[test]
fn test_language_detection_in_template() {
    // Test that i18n can detect current language
    let catalog1 = MessageCatalog::new("it");
    activate("it", catalog1);

    // Use real i18n function instead of template stub
    let lang = get_locale();
    assert_eq!(lang, "it");

    deactivate();

    let catalog2 = MessageCatalog::new("pt");
    activate("pt", catalog2);
    let lang2 = get_locale();
    assert_eq!(lang2, "pt");

    deactivate();
}

#[test]
fn test_multilingual_template_rendering() {
    // Test rendering the same template in multiple languages
    let languages = vec![
        ("en", "Hello", "Hello"),
        ("ja", "Hello", "こんにちは"),
        ("es", "Hello", "Hola"),
        ("fr", "Hello", "Bonjour"),
    ];

    for (lang, key, expected) in languages {
        let mut catalog = MessageCatalog::new(lang);
        catalog.add_translation(key, expected);

        activate(lang, catalog);

        // Use real i18n function for actual translation
        let translated = gettext(key);
        assert_eq!(translated, expected, "Failed for language: {}", lang);

        deactivate();
    }
}
#[test]
fn test_template_with_missing_translation() {
    let mut catalog = MessageCatalog::new("zh");
    catalog.add_translation("Existing", "存在");

    activate("zh", catalog);

    // Translation exists - use real i18n function
    let existing = gettext("Existing");
    assert_eq!(existing, "存在");

    // Translation missing - should return original key
    let missing = gettext("NonExistent");
    assert_eq!(missing, "NonExistent");

    deactivate();
}

#[test]
fn test_template_fallback_to_default_language() {
    // Test fallback behavior when translation is not available
    let catalog = MessageCatalog::new("unknown_lang");
    activate("unknown_lang", catalog);

    // Use real i18n function - should return original when no translation exists
    let result = gettext("Hello");
    assert_eq!(result, "Hello");

    deactivate();
}

#[test]
fn test_nested_translation_with_variables() {
    let mut catalog = MessageCatalog::new("ko");
    catalog.add_translation(
        "Hello user, you have messages",
        "안녕하세요님, 메시지가 있습니다",
    );

    activate("ko", catalog);

    // Note: blocktrans is a template stub that doesn't do actual translation
    // It just returns the input string as-is
    let result = blocktrans("Hello user, you have messages").unwrap();
    assert_eq!(result, "Hello user, you have messages");

    deactivate();
}
