//! i18n template filters
//!
//! Translation and localization filters for templates

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use reinhardt_exception::{Error, Result};

/// Get the current language
///
/// # Example
/// ```
/// use reinhardt_templates::get_current_language;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// // Default language
/// assert_eq!(get_current_language(), "en-US");
///
/// // Change language
/// let catalog = MessageCatalog::new("ja");
/// activate_with_catalog("ja", catalog);
/// assert_eq!(get_current_language(), "ja");
/// ```
pub fn get_current_language() -> String {
	reinhardt_i18n::get_language()
}

/// Translate a string
///
/// # Example
/// ```
/// use reinhardt_templates::trans;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("es");
/// catalog.add_translation("Hello", "Hola");
/// activate_with_catalog("es", catalog);
///
/// let result = trans("Hello").unwrap();
/// assert_eq!(result, "Hola");
/// ```
pub fn trans(message: &str) -> Result<String> {
	Ok(reinhardt_i18n::gettext(message))
}

/// Translate a string with context
///
/// # Example
/// ```
/// use reinhardt_templates::trans_with_context;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("de");
/// catalog.add_context_str("menu", "File", "Datei");
/// catalog.add_context_str("verb", "File", "Ablegen");
/// activate_with_catalog("de", catalog);
///
/// let menu = trans_with_context("menu", "File").unwrap();
/// assert_eq!(menu, "Datei");
///
/// let verb = trans_with_context("verb", "File").unwrap();
/// assert_eq!(verb, "Ablegen");
/// ```
pub fn trans_with_context(context: &str, message: &str) -> Result<String> {
	Ok(reinhardt_i18n::pgettext(context, message))
}

/// Block translation
///
/// Translates a block of text using the active translation catalog.
/// If no translation is found, returns the original message.
///
/// # Example
/// ```
/// use reinhardt_templates::blocktrans;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("fr");
/// catalog.add_translation("Welcome!", "Bienvenue!");
/// activate_with_catalog("fr", catalog);
///
/// let result = blocktrans("Welcome!").unwrap();
/// assert_eq!(result, "Bienvenue!");
/// ```
pub fn blocktrans(message: &str) -> Result<String> {
	Ok(reinhardt_i18n::gettext(message))
}

/// Block translation with plural support
///
/// Translates text with automatic plural form selection based on count.
/// Uses language-specific plural rules from the active catalog.
///
/// # Example
/// ```
/// use reinhardt_templates::blocktrans_plural;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("ru");
/// catalog.add_plural_str("item", "items", vec!["предмет", "предмета", "предметов"]);
/// activate_with_catalog("ru", catalog);
///
/// // Translate according to Russian plural rules
/// let one = blocktrans_plural("item", "items", 1).unwrap();
/// assert_eq!(one, "предмет");
///
/// let few = blocktrans_plural("item", "items", 2).unwrap();
/// assert_eq!(few, "предмета");
/// ```
pub fn blocktrans_plural(singular: &str, plural: &str, count: usize) -> Result<String> {
	Ok(reinhardt_i18n::ngettext(singular, plural, count))
}

/// Translate with context and plural support
///
/// Combines context-aware translation with plural forms.
///
/// # Example
/// ```
/// use reinhardt_templates::trans_plural_with_context;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("pl");
/// catalog.add_context_plural("email", "message", "messages", vec!["wiadomość", "wiadomości"]);
/// catalog.add_context_plural("sms", "message", "messages", vec!["SMS", "SMS-y"]);
/// activate_with_catalog("pl", catalog);
///
/// let email_one = trans_plural_with_context("email", "message", "messages", 1).unwrap();
/// assert_eq!(email_one, "wiadomość");
///
/// let sms_many = trans_plural_with_context("sms", "message", "messages", 5).unwrap();
/// assert_eq!(sms_many, "SMS-y");
/// ```
pub fn trans_plural_with_context(
	context: &str,
	singular: &str,
	plural: &str,
	count: usize,
) -> Result<String> {
	Ok(reinhardt_i18n::npgettext(context, singular, plural, count))
}

// ============================================================================
// Date/Time Formatting
// ============================================================================

/// Format a date according to the current locale
///
/// Supports ISO 8601 date strings and timestamps.
///
/// # Example
/// ```
/// use reinhardt_templates::localize_date_filter;
///
/// // ISO 8601 format date
/// let result = localize_date_filter("2024-03-15").unwrap();
/// assert!(result.contains("2024"));
/// ```
pub fn localize_date_filter(date: &str) -> Result<String> {
	let locale = reinhardt_i18n::get_language();

	// Parse ISO 8601 date format
	if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
		return format_date_for_locale(&naive_date, &locale);
	}

	// Parse ISO 8601 datetime format
	if let Ok(naive_datetime) = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S") {
		return format_datetime_for_locale(&naive_datetime, &locale);
	}

	// Parse RFC 3339 format
	if let Ok(datetime) = DateTime::parse_from_rfc3339(date) {
		return format_datetime_for_locale(&datetime.naive_local(), &locale);
	}

	// Return original string if parsing fails
	Ok(date.to_string())
}

/// Format a date with a custom format string
///
/// # Example
/// ```
/// use reinhardt_templates::localize_date_with_format;
///
/// let result = localize_date_with_format("2024-03-15", "%Y年%m月%d日").unwrap();
/// assert_eq!(result, "2024年03月15日");
/// ```
pub fn localize_date_with_format(date: &str, format: &str) -> Result<String> {
	if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
		return Ok(naive_date.format(format).to_string());
	}

	if let Ok(naive_datetime) = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S") {
		return Ok(naive_datetime.format(format).to_string());
	}

	Err(Error::Validation(format!("Invalid date format: {}", date)))
}

/// Format date for specific locale
fn format_date_for_locale(date: &NaiveDate, locale: &str) -> Result<String> {
	let format = match locale {
		l if l.starts_with("ja") => "%Y年%m月%d日",
		l if l.starts_with("zh") => "%Y年%m月%d日",
		l if l.starts_with("ko") => "%Y년 %m월 %d일",
		l if l.starts_with("en-US") => "%m/%d/%Y",
		l if l.starts_with("en-GB") => "%d/%m/%Y",
		l if l.starts_with("de") => "%d.%m.%Y",
		l if l.starts_with("fr") => "%d/%m/%Y",
		l if l.starts_with("es") => "%d/%m/%Y",
		l if l.starts_with("it") => "%d/%m/%Y",
		_ => "%Y-%m-%d", // ISO 8601 default
	};

	Ok(date.format(format).to_string())
}

/// Format datetime for specific locale
fn format_datetime_for_locale(datetime: &NaiveDateTime, locale: &str) -> Result<String> {
	let format = match locale {
		l if l.starts_with("ja") => "%Y年%m月%d日 %H:%M:%S",
		l if l.starts_with("zh") => "%Y年%m月%d日 %H:%M:%S",
		l if l.starts_with("ko") => "%Y년 %m월 %d일 %H:%M:%S",
		l if l.starts_with("en-US") => "%m/%d/%Y %I:%M:%S %p",
		l if l.starts_with("en-GB") => "%d/%m/%Y %H:%M:%S",
		l if l.starts_with("de") => "%d.%m.%Y %H:%M:%S",
		l if l.starts_with("fr") => "%d/%m/%Y %H:%M:%S",
		l if l.starts_with("es") => "%d/%m/%Y %H:%M:%S",
		l if l.starts_with("it") => "%d/%m/%Y %H:%M:%S",
		_ => "%Y-%m-%d %H:%M:%S", // ISO 8601 default
	};

	Ok(datetime.format(format).to_string())
}

// ============================================================================
// Number Formatting
// ============================================================================

/// Format a number according to the current locale
///
/// # Example
/// ```
/// use reinhardt_templates::localize_number_filter;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// // Default locale (en-US)
/// let result = localize_number_filter(1234567.89).unwrap();
/// assert_eq!(result, "1,234,567.89");
///
/// // German locale
/// let catalog = MessageCatalog::new("de");
/// activate_with_catalog("de", catalog);
/// let result = localize_number_filter(1234567.89).unwrap();
/// assert_eq!(result, "1.234.567,89");
/// ```
pub fn localize_number_filter(number: f64) -> Result<String> {
	let locale = reinhardt_i18n::get_language();
	format_number_for_locale(number, &locale)
}

/// Format an integer according to the current locale
///
/// # Example
/// ```
/// use reinhardt_templates::localize_integer_filter;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// let catalog = MessageCatalog::new("fr");
/// activate_with_catalog("fr", catalog);
///
/// let result = localize_integer_filter(1234567).unwrap();
/// assert_eq!(result, "1 234 567");
/// ```
pub fn localize_integer_filter(number: i64) -> Result<String> {
	let locale = reinhardt_i18n::get_language();
	format_integer_for_locale(number, &locale)
}

/// Format number for specific locale
fn format_number_for_locale(number: f64, locale: &str) -> Result<String> {
	let (thousands_sep, decimal_sep) = get_locale_separators(locale);

	// Separate integer and decimal parts
	let number_str = number.to_string();
	let parts: Vec<&str> = number_str.split('.').collect();
	let integer_part = parts[0];
	let decimal_part = if parts.len() > 1 { parts[1] } else { "" };

	// Add thousands separator to integer part
	let formatted_integer = add_thousands_separator(integer_part, thousands_sep);

	// Combine with decimal part if present
	if !decimal_part.is_empty() {
		Ok(format!(
			"{}{}{}",
			formatted_integer, decimal_sep, decimal_part
		))
	} else {
		Ok(formatted_integer)
	}
}

/// Format integer for specific locale
fn format_integer_for_locale(number: i64, locale: &str) -> Result<String> {
	let (thousands_sep, _) = get_locale_separators(locale);
	let integer_str = number.to_string();
	Ok(add_thousands_separator(&integer_str, thousands_sep))
}

/// Get thousands and decimal separators for locale
fn get_locale_separators(locale: &str) -> (&'static str, &'static str) {
	match locale {
		l if l.starts_with("de") => (".", ","), // German: 1.234.567,89
		l if l.starts_with("fr") => (" ", ","), // French: 1 234 567,89
		l if l.starts_with("es") => (".", ","), // Spanish: 1.234.567,89
		l if l.starts_with("it") => (".", ","), // Italian: 1.234.567,89
		l if l.starts_with("ru") => (" ", ","), // Russian: 1 234 567,89
		l if l.starts_with("ja") => (",", "."), // Japanese: 1,234,567.89
		l if l.starts_with("zh") => (",", "."), // Chinese: 1,234,567.89
		l if l.starts_with("ko") => (",", "."), // Korean: 1,234,567.89
		_ => (",", "."),                        // Default (English): 1,234,567.89
	}
}

/// Add thousands separator to integer string
fn add_thousands_separator(num_str: &str, separator: &str) -> String {
	let is_negative = num_str.starts_with('-');
	let digits = if is_negative { &num_str[1..] } else { num_str };

	let mut result = String::new();
	let len = digits.len();

	for (i, ch) in digits.chars().enumerate() {
		if i > 0 && (len - i) % 3 == 0 {
			result.push_str(separator);
		}
		result.push(ch);
	}

	if is_negative {
		format!("-{}", result)
	} else {
		result
	}
}

/// Format currency according to locale
///
/// # Example
/// ```
/// use reinhardt_templates::localize_currency_filter;
/// use reinhardt_i18n::{activate_with_catalog, MessageCatalog};
///
/// // US Dollar
/// let catalog = MessageCatalog::new("en-US");
/// activate_with_catalog("en-US", catalog);
/// let result = localize_currency_filter(1234.56, "USD").unwrap();
/// assert_eq!(result, "$1,234.56");
///
/// // Euro (German)
/// let catalog = MessageCatalog::new("de");
/// activate_with_catalog("de", catalog);
/// let result = localize_currency_filter(1234.56, "EUR").unwrap();
/// assert_eq!(result, "1.234,56 €");
/// ```
pub fn localize_currency_filter(amount: f64, currency: &str) -> Result<String> {
	let locale = reinhardt_i18n::get_language();
	format_currency_for_locale(amount, currency, &locale)
}

/// Format currency for specific locale
fn format_currency_for_locale(amount: f64, currency: &str, locale: &str) -> Result<String> {
	let formatted_amount = format_number_for_locale(amount, locale)?;

	let currency_symbol = match currency {
		"USD" => "$",
		"EUR" => "€",
		"GBP" => "£",
		"JPY" => "¥",
		"CNY" => "¥",
		"KRW" => "₩",
		_ => currency,
	};

	// Change currency symbol position based on locale
	match locale {
		l if l.starts_with("en-US") => Ok(format!("{}{}", currency_symbol, formatted_amount)),
		l if l.starts_with("en-GB") => Ok(format!("{}{}", currency_symbol, formatted_amount)),
		l if l.starts_with("de")
			| l.starts_with("fr")
			| l.starts_with("es")
			| l.starts_with("it") =>
		{
			Ok(format!("{} {}", formatted_amount, currency_symbol))
		}
		l if l.starts_with("ja") | l.starts_with("zh") | l.starts_with("ko") => {
			Ok(format!("{}{}", currency_symbol, formatted_amount))
		}
		_ => Ok(format!("{} {}", formatted_amount, currency)),
	}
}
