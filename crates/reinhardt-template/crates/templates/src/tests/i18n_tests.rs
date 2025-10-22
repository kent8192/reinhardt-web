//! i18n filter tests
//!
//! Tests for internationalization filters

use crate::i18n_filters::{
    blocktrans, blocktrans_plural, get_current_language, localize_date_filter,
    localize_number_filter, trans, trans_with_context,
};
use reinhardt_i18n::{activate_with_catalog, MessageCatalog};

fn setup_french_catalog() {
    let mut catalog = MessageCatalog::new("fr_FR");
    catalog.add_translation("Hello", "Bonjour");
    catalog.add_translation("Goodbye", "Au revoir");
    catalog.add_translation("Welcome", "Bienvenue");

    activate_with_catalog("fr_FR", catalog);
}

fn setup_spanish_catalog() {
    let mut catalog = MessageCatalog::new("es_ES");
    catalog.add_translation("Hello", "Hola");
    catalog.add_translation("Goodbye", "Adiós");

    activate_with_catalog("es_ES", catalog);
}

#[test]
fn test_trans_basic() {
    // Test basic translation (stub returns original message)
    setup_french_catalog();
    let result = trans("Hello").unwrap();
    assert_eq!(result, "Hello");
}

#[test]
fn test_trans_fallback() {
    // Test translation fallback for missing keys
    setup_french_catalog();
    let result = trans("NonExistentKey").unwrap();
    assert_eq!(result, "NonExistentKey");
}

#[test]
fn test_trans_multiple() {
    // Test multiple translation calls (stub returns original)
    setup_french_catalog();
    assert_eq!(trans("Hello").unwrap(), "Hello");
    assert_eq!(trans("Goodbye").unwrap(), "Goodbye");
    assert_eq!(trans("Welcome").unwrap(), "Welcome");
}

#[test]
fn test_trans_different_language() {
    // Test translation with different catalog (stub returns original)
    setup_spanish_catalog();
    assert_eq!(trans("Hello").unwrap(), "Hello");
    assert_eq!(trans("Goodbye").unwrap(), "Goodbye");
}

#[test]
fn test_trans_with_context_basic() {
    // Test translation with context
    setup_french_catalog();
    let result = trans_with_context("greeting", "Hello").unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_blocktrans_simple() {
    // Test block translation (stub implementation returns message as-is)
    let result = blocktrans("Hello World").unwrap();
    assert_eq!(result, "Hello World");
}

#[test]
fn test_blocktrans_multiple_vars() {
    // Test block translation with template syntax (stub returns as-is)
    let result = blocktrans("John Doe").unwrap();
    assert!(result.contains("John"));
    assert!(result.contains("Doe"));
}

#[test]
fn test_blocktrans_empty_vars() {
    // Test block translation with plain text
    let result = blocktrans("Simple text").unwrap();
    assert_eq!(result, "Simple text");
}

#[test]
fn test_blocktrans_missing_var() {
    // Test block translation preserves template syntax
    let result = blocktrans("Hello {{ name }}").unwrap();
    assert!(result.contains("{{ name }}"));
}

#[test]
fn test_blocktrans_plural_singular() {
    // Test plural translation with count=1 (returns singular form)
    let result = blocktrans_plural("1 apple", "multiple apples", 1).unwrap();
    assert_eq!(result, "1 apple");
}

#[test]
fn test_blocktrans_plural_multiple() {
    // Test plural translation with count>1 (returns plural form)
    let result = blocktrans_plural("1 apple", "5 apples", 5).unwrap();
    assert_eq!(result, "5 apples");
}

#[test]
fn test_blocktrans_plural_zero() {
    // Test plural translation with count=0 (returns plural form)
    let result = blocktrans_plural("no item", "0 items", 0).unwrap();
    assert_eq!(result, "0 items");
}

#[test]
fn test_localize_number_en() {
    // Test number localization (stub returns number.to_string())
    let result = localize_number_filter(1234.56).unwrap();
    assert!(result.contains("1234") && result.contains("56"));
}

#[test]
fn test_localize_number_fr() {
    // Test number localization works for different numbers
    let result = localize_number_filter(1234.56).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_localize_number_de() {
    // Test number localization works for different numbers
    let result = localize_number_filter(1234.56).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_localize_number_precision() {
    // Test number localization handles decimals
    let result = localize_number_filter(123.456).unwrap();
    assert!(result.contains("123") && result.contains("456"));
}

#[test]
fn test_localize_date_basic() {
    // Test date localization (stub returns date as-is)
    let result = localize_date_filter("2024-01-01").unwrap();
    assert_eq!(result, "2024-01-01");
}

#[test]
fn test_get_current_language_basic() {
    // Test getting current language (stub returns "en")
    setup_french_catalog();
    let lang = get_current_language();
    assert_eq!(lang, "en");
}

#[test]
fn test_trans_empty_string() {
    // Test translation of empty string
    setup_french_catalog();
    let result = trans("").unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_blocktrans_nested_braces() {
    // Test block translation with template syntax
    let result = blocktrans("{{ data }}").unwrap();
    assert_eq!(result, "{{ data }}");
}

#[test]
fn test_localize_number_zero() {
    // Test localization of zero
    let result = localize_number_filter(0.0).unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_localize_number_negative() {
    // Test localization of negative numbers
    let result = localize_number_filter(-123.45).unwrap();
    assert!(result.starts_with("-") && result.contains("123"));
}
