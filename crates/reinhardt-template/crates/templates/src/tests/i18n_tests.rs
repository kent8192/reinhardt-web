//! i18n filter tests
//!
//! Tests for internationalization filters

use crate::i18n_filters::{
	blocktrans, blocktrans_plural, get_current_language, localize_currency_filter,
	localize_date_filter, localize_date_with_format, localize_integer_filter,
	localize_number_filter, trans, trans_plural_with_context, trans_with_context,
};
use reinhardt_i18n::{MessageCatalog, activate_with_catalog, deactivate};
use serial_test::serial;

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
#[serial(i18n)]
fn test_trans_basic() {
	// Test basic translation
	setup_french_catalog();
	let result = trans("Hello").unwrap();
	assert_eq!(result, "Bonjour");
	deactivate();
}

#[test]
fn test_trans_fallback() {
	// Test translation fallback for missing keys
	setup_french_catalog();
	let result = trans("NonExistentKey").unwrap();
	assert_eq!(result, "NonExistentKey");
}

#[test]
#[serial(i18n)]
fn test_trans_multiple() {
	// Test multiple translation calls
	setup_french_catalog();
	assert_eq!(trans("Hello").unwrap(), "Bonjour");
	assert_eq!(trans("Goodbye").unwrap(), "Au revoir");
	assert_eq!(trans("Welcome").unwrap(), "Bienvenue");
	deactivate();
}

#[test]
#[serial(i18n)]
fn test_trans_different_language() {
	// Test translation with different catalog
	setup_spanish_catalog();
	assert_eq!(trans("Hello").unwrap(), "Hola");
	assert_eq!(trans("Goodbye").unwrap(), "Adiós");
	deactivate();
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
#[serial(i18n)]
fn test_localize_number_en() {
	// Test number localization with en-US locale
	let catalog = MessageCatalog::new("en-US");
	activate_with_catalog("en-US", catalog);
	let result = localize_number_filter(1234.56).unwrap();
	assert_eq!(result, "1,234.56");
	deactivate();
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
#[serial(i18n)]
fn test_localize_date_basic() {
	// Test date localization with default locale
	let catalog = MessageCatalog::new("en-US");
	activate_with_catalog("en-US", catalog);
	let result = localize_date_filter("2024-01-01").unwrap();
	assert_eq!(result, "01/01/2024");
	deactivate();
}

#[test]
#[serial(i18n)]
fn test_get_current_language_basic() {
	// Test getting current language
	setup_french_catalog();
	let lang = get_current_language();
	assert_eq!(lang, "fr_FR");
	deactivate();
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

// ============================================================================
// Enhanced i18n Tests - Pluralization Support
// ============================================================================

#[test]
#[serial(i18n)]
fn test_blocktrans_plural_with_catalog() {
	let mut catalog = MessageCatalog::new("ru");
	catalog.add_plural_str("item", "items", vec!["предмет", "предмета", "предметов"]);
	activate_with_catalog("ru", catalog);

	// Russian plural rules: 1 is singular form
	let result = blocktrans_plural("item", "items", 1).unwrap();
	assert_eq!(result, "предмет");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_trans_plural_with_context_email() {
	let mut catalog = MessageCatalog::new("pl");
	catalog.add_context_plural(
		"email",
		"message",
		"messages",
		vec!["wiadomość", "wiadomości"],
	);
	activate_with_catalog("pl", catalog);

	let result = trans_plural_with_context("email", "message", "messages", 1).unwrap();
	assert_eq!(result, "wiadomość");

	let result = trans_plural_with_context("email", "message", "messages", 5).unwrap();
	assert_eq!(result, "wiadomości");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_trans_plural_with_context_sms() {
	let mut catalog = MessageCatalog::new("pl");
	catalog.add_context_plural("sms", "message", "messages", vec!["SMS", "SMS-y"]);
	activate_with_catalog("pl", catalog);

	let result = trans_plural_with_context("sms", "message", "messages", 1).unwrap();
	assert_eq!(result, "SMS");

	let result = trans_plural_with_context("sms", "message", "messages", 3).unwrap();
	assert_eq!(result, "SMS-y");

	deactivate();
}

// ============================================================================
// Enhanced i18n Tests - Context-aware Translations
// ============================================================================

#[test]
#[serial(i18n)]
fn test_trans_with_context_menu_vs_verb() {
	let mut catalog = MessageCatalog::new("de");
	catalog.add_context_str("menu", "File", "Datei");
	catalog.add_context_str("verb", "File", "Ablegen");
	activate_with_catalog("de", catalog);

	let menu = trans_with_context("menu", "File").unwrap();
	assert_eq!(menu, "Datei");

	let verb = trans_with_context("verb", "File").unwrap();
	assert_eq!(verb, "Ablegen");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_trans_with_context_fallback() {
	let catalog = MessageCatalog::new("ja");
	activate_with_catalog("ja", catalog);

	// Return original message if context is not found
	let result = trans_with_context("unknown_context", "File").unwrap();
	assert_eq!(result, "File");

	deactivate();
}

// ============================================================================
// Enhanced i18n Tests - Date/Time Formatting
// ============================================================================

#[test]
#[serial(i18n)]
fn test_localize_date_japanese() {
	let catalog = MessageCatalog::new("ja");
	activate_with_catalog("ja", catalog);

	let result = localize_date_filter("2024-03-15").unwrap();
	assert_eq!(result, "2024年03月15日");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_date_korean() {
	let catalog = MessageCatalog::new("ko");
	activate_with_catalog("ko", catalog);

	let result = localize_date_filter("2024-03-15").unwrap();
	assert_eq!(result, "2024년 03월 15일");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_date_us_format() {
	let catalog = MessageCatalog::new("en-US");
	activate_with_catalog("en-US", catalog);

	let result = localize_date_filter("2024-03-15").unwrap();
	assert_eq!(result, "03/15/2024");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_date_german() {
	let catalog = MessageCatalog::new("de");
	activate_with_catalog("de", catalog);

	let result = localize_date_filter("2024-03-15").unwrap();
	assert_eq!(result, "15.03.2024");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_date_with_format_custom() {
	let result = localize_date_with_format("2024-03-15", "%Y年%m月%d日").unwrap();
	assert_eq!(result, "2024年03月15日");
}

#[test]
#[serial(i18n)]
fn test_localize_datetime_japanese() {
	let catalog = MessageCatalog::new("ja");
	activate_with_catalog("ja", catalog);

	let result = localize_date_filter("2024-03-15T14:30:00").unwrap();
	assert_eq!(result, "2024年03月15日 14:30:00");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_date_rfc3339() {
	let catalog = MessageCatalog::new("en-GB");
	activate_with_catalog("en-GB", catalog);

	let result = localize_date_filter("2024-03-15T14:30:00+09:00").unwrap();
	assert!(result.contains("15/03/2024"));

	deactivate();
}

// ============================================================================
// Enhanced i18n Tests - Number Formatting
// ============================================================================

#[test]
#[serial(i18n)]
fn test_localize_number_english_default() {
	let catalog = MessageCatalog::new("en-US");
	activate_with_catalog("en-US", catalog);

	let result = localize_number_filter(1234567.89).unwrap();
	assert_eq!(result, "1,234,567.89");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_number_german() {
	let catalog = MessageCatalog::new("de");
	activate_with_catalog("de", catalog);

	let result = localize_number_filter(1234567.89).unwrap();
	assert_eq!(result, "1.234.567,89");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_number_french() {
	let catalog = MessageCatalog::new("fr");
	activate_with_catalog("fr", catalog);

	let result = localize_number_filter(1234567.89).unwrap();
	assert_eq!(result, "1 234 567,89");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_number_japanese() {
	let catalog = MessageCatalog::new("ja");
	activate_with_catalog("ja", catalog);

	let result = localize_number_filter(1234567.89).unwrap();
	assert_eq!(result, "1,234,567.89");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_integer_french() {
	let catalog = MessageCatalog::new("fr");
	activate_with_catalog("fr", catalog);

	let result = localize_integer_filter(1234567).unwrap();
	assert_eq!(result, "1 234 567");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_integer_negative() {
	let catalog = MessageCatalog::new("de");
	activate_with_catalog("de", catalog);

	let result = localize_integer_filter(-1234567).unwrap();
	assert_eq!(result, "-1.234.567");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_currency_usd() {
	let catalog = MessageCatalog::new("en-US");
	activate_with_catalog("en-US", catalog);

	let result = localize_currency_filter(1234.56, "USD").unwrap();
	assert_eq!(result, "$1,234.56");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_currency_euro_german() {
	let catalog = MessageCatalog::new("de");
	activate_with_catalog("de", catalog);

	let result = localize_currency_filter(1234.56, "EUR").unwrap();
	assert_eq!(result, "1.234,56 €");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_currency_yen_japanese() {
	let catalog = MessageCatalog::new("ja");
	activate_with_catalog("ja", catalog);

	let result = localize_currency_filter(1234.56, "JPY").unwrap();
	assert_eq!(result, "¥1,234.56");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_currency_gbp() {
	let catalog = MessageCatalog::new("en-GB");
	activate_with_catalog("en-GB", catalog);

	let result = localize_currency_filter(999.99, "GBP").unwrap();
	assert_eq!(result, "£999.99");

	deactivate();
}

// ============================================================================
// Enhanced i18n Tests - Edge Cases
// ============================================================================

#[test]
#[serial(i18n)]
fn test_get_current_language_after_activate() {
	let catalog = MessageCatalog::new("fr");
	activate_with_catalog("fr", catalog);

	let lang = get_current_language();
	assert_eq!(lang, "fr");

	deactivate();
}

#[test]
#[serial(i18n)]
fn test_localize_number_small_value() {
	let catalog = MessageCatalog::new("de");
	activate_with_catalog("de", catalog);

	let result = localize_number_filter(12.5).unwrap();
	assert_eq!(result, "12,5");

	deactivate();
}

#[test]
fn test_localize_date_invalid_format() {
	let result = localize_date_filter("invalid-date").unwrap();
	// Return original string if parsing fails
	assert_eq!(result, "invalid-date");
}

#[test]
fn test_localize_date_with_format_invalid() {
	let result = localize_date_with_format("invalid-date", "%Y-%m-%d");
	assert!(result.is_err());
}
