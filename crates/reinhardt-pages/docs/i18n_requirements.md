# I18N Requirements Document

## Background

The deleted `reinhardt-template` crate contained approximately 1,414 lines of i18n tests.
This document extracts requirements from those tests and serves as a guideline for future i18n implementation in `reinhardt-pages`.

## Purpose

Integrate internationalization (i18n) functionality into the `reinhardt-pages` framework to enable building multi-language WASM applications.

---

## Core Requirements

### 1. Basic Translation Features

#### 1.1 Simple Translation (`gettext`)
```rust
use reinhardt_pages::i18n::{gettext, _};

let message = gettext("welcome_message");
// Or shorthand
let message = _("welcome_message");
```

**Requirements**:
- Retrieve translation strings from message IDs
- Return message ID as-is if translation not found
- Support runtime locale switching

#### 1.2 Plural Forms Translation (`ngettext`)
```rust
use reinhardt_pages::i18n::ngettext;

let message = ngettext(
	"You have {count} message",
	"You have {count} messages",
	count
);
```

**Requirements**:
- Select singular/plural form based on count
- Support language-specific plural rules (CLDR compliant)
  - English: 2 forms (1, other)
  - Japanese: 1 form (always same)
  - Russian: 3 forms (1, 2-4, other)
  - Arabic: 6 forms
- Handle zero differently per language

#### 1.3 Context-Aware Translation (`pgettext`)
```rust
use reinhardt_pages::i18n::pgettext;

let month = pgettext("calendar", "May");  // May as in the month
let modal = pgettext("permission", "May");  // May as in permission
```

**Requirements**:
- Provide different translations for same word based on context
- Manage via combination of context ID and message ID

---

### 2. Locale Management

#### 2.1 Locale Detection
```rust
use reinhardt_pages::i18n::Locale;

// Auto-detect from browser's Accept-Language header
let locale = Locale::from_browser_default();

// Explicitly specify
let locale = Locale::new("ja-JP");
```

**Requirements**:
- Auto-detect from browser's Accept-Language header
- Override with user settings (Cookie, LocalStorage)
- Fallback chain: `ja-JP` → `ja` → `en` → default

#### 2.2 Dynamic Locale Switching
```rust
use reinhardt_pages::i18n::set_locale;

// Change locale
set_locale(Locale::new("fr-FR"));

// Update reactively
let locale_signal = use_locale();
locale_signal.set(Locale::new("de-DE"));
```

**Requirements**:
- Runtime locale switching
- Signal-based reactive updates
- All UI components auto-update on switch

#### 2.3 Locale Information Retrieval
```rust
let current_locale = get_locale();
let language_code = current_locale.language(); // "ja"
let country_code = current_locale.country();   // "JP"
let full_code = current_locale.code();         // "ja-JP"
```

---

### 3. String Interpolation

#### 3.1 Named Placeholders
```rust
let message = _("Hello, {name}!").format([("name", "Alice")]);
// "Hello, Alice!"

let message = _("Order #{order_id} for {customer}").format([
	("order_id", "12345"),
	("customer", "Bob"),
]);
// "Order #12345 for Bob"
```

**Requirements**:
- `{name}` style placeholders
- Support multiple placeholders
- Warn if placeholder not found

#### 3.2 Positional Placeholders
```rust
let message = _("Item {0} of {1}").format_positional([1, 10]);
// "Item 1 of 10"
```

**Requirements**:
- `{0}`, `{1}`, `{2}` style placeholders
- 0-indexed

#### 3.3 Reactive Value Interpolation
```rust
let count = Signal::new(0);
let message = _("You have {count} items").format_signal([
	("count", count.clone())
]);

// Message auto-updates when count changes
count.set(5); // message = "You have 5 items"
```

**Requirements**:
- Use `Signal<T>` in placeholders
- Translation string auto-updates when Signal value changes
- Work reactively with Effects

---

### 4. Localization (Formatting)

#### 4.1 Date/Time Formatting
```rust
use reinhardt_pages::i18n::format_date;

let date = DateTime::from_timestamp(1640000000, 0);

// Japanese
set_locale(Locale::new("ja-JP"));
format_date(&date, "long");    // "2021年12月20日"
format_date(&date, "short");   // "2021/12/20"
format_time(&date, "long");    // "午後6時26分40秒"

// English
set_locale(Locale::new("en-US"));
format_date(&date, "long");    // "December 20, 2021"
format_date(&date, "short");   // "12/20/2021"
format_time(&date, "long");    // "6:26:40 PM"
```

**Requirements**:
- CLDR-compliant formatting
- Support custom format strings
- Timezone support

#### 4.2 Number Formatting
```rust
use reinhardt_pages::i18n::format_number;

let value = 1234567.89;

// Japanese
set_locale(Locale::new("ja-JP"));
format_number(value);  // "1,234,567.89"

// German (reversed period and comma)
set_locale(Locale::new("de-DE"));
format_number(value);  // "1.234.567,89"

// Arabic (Arabic numerals)
set_locale(Locale::new("ar-SA"));
format_number(value);  // "١٬٢٣٤٬٥٦٧٫٨٩"
```

**Requirements**:
- Thousand separators (comma, period, space, etc.)
- Decimal separators (period, comma)
- Numeral systems (Arabic, Arabic-Indic, etc.)

#### 4.3 Currency Formatting
```rust
use reinhardt_pages::i18n::format_currency;

let amount = 1234.56;

// Japanese Yen
set_locale(Locale::new("ja-JP"));
format_currency(amount, "JPY");  // "¥1,235"

// US Dollar
set_locale(Locale::new("en-US"));
format_currency(amount, "USD");  // "$1,234.56"

// Euro (France)
set_locale(Locale::new("fr-FR"));
format_currency(amount, "EUR");  // "1 234,56 €"
```

**Requirements**:
- Currency symbol position (before, after)
- Decimal places (varies by currency)
- Negative value display (`-$10`, `($10)`, `$-10`, etc.)

---

## Proposed Implementation Architecture

### Frontend (WASM)

```rust
use reinhardt_pages::i18n::{I18n, Locale, _};
use reinhardt_pages::component::{Component, View};
use reinhardt_pages::reactive::Signal;

struct AppComponent {
	locale: Signal<Locale>,
}

impl Component for AppComponent {
	fn render(&self) -> View {
		View::element("div")
			.child(View::element("h1").child(_("welcome_title")))
			.child(View::element("p").child(_("welcome_message")))
			.child(self.render_language_switcher())
			.into_view()
	}

	fn name() -> &'static str {
		"AppComponent"
	}
}

impl AppComponent {
	fn render_language_switcher(&self) -> View {
		View::element("select")
			.on("change", move |event| {
				let lang = event.target_value();
				self.locale.set(Locale::new(&lang));
			})
			.child(View::element("option").attr("value", "en").child("English"))
			.child(View::element("option").attr("value", "ja").child("日本語"))
			.child(View::element("option").attr("value", "fr").child("Français"))
			.into_view()
	}
}
```

### Backend (Translation Data Provider)

#### Translation File Format (JSON)
```json
{
  "locale": "ja-JP",
  "messages": {
	"welcome_title": "ようこそ",
	"welcome_message": "Reinhardtフレームワークへようこそ！",
	"items_count": {
	  "one": "{count}個のアイテム",
	  "other": "{count}個のアイテム"
	}
  },
  "contexts": {
	"calendar": {
	  "May": "5月"
	},
	"permission": {
	  "May": "できる"
	}
  }
}
```

#### Build-Time Compilation
```rust
// build.rs
use reinhardt_i18n_build::compile_translations;

fn main() {
	compile_translations("locales/")
		.output("src/generated/translations.rs")
		.compile();
}
```

#### Server-Side Fetch
```rust
use reinhardt_pages::i18n::load_translations;

async fn initialize_i18n() {
	// Fetch translation data from server
	let translations = fetch("/api/i18n/ja-JP").await?;
	load_translations(translations).await?;
}
```

---

## Test Case Examples

### Basic Translation
```rust
#[test]
fn test_basic_translation() {
	set_locale(Locale::new("ja-JP"));
	load_translations(japanese_translations());

	assert_eq!(_("welcome"), "ようこそ");
	assert_eq!(_("goodbye"), "さようなら");
}
```

### Plural Forms
```rust
#[test]
fn test_plural_forms() {
	set_locale(Locale::new("en-US"));

	assert_eq!(ngettext("1 item", "{count} items", 0).format([("count", 0)]), "0 items");
	assert_eq!(ngettext("1 item", "{count} items", 1).format([("count", 1)]), "1 item");
	assert_eq!(ngettext("1 item", "{count} items", 5).format([("count", 5)]), "5 items");
}
```

### Locale Fallback
```rust
#[test]
fn test_locale_fallback() {
	set_locale(Locale::new("ja-JP"));

	// Fallback to ja if not in ja-JP
	assert_eq!(_("only_in_ja"), "日本語のみ");

	// Fallback to en if not in ja
	assert_eq!(_("only_in_en"), "English only");

	// Return message ID if not found anywhere
	assert_eq!(_("missing_key"), "missing_key");
}
```

### Reactive Updates
```rust
#[test]
fn test_reactive_locale_change() {
	let locale = Signal::new(Locale::new("en-US"));
	set_locale_signal(locale.clone());

	let message = use_translation("welcome");
	assert_eq!(message.get(), "Welcome");

	locale.set(Locale::new("ja-JP"));
	assert_eq!(message.get(), "ようこそ");
}
```

---

## Implementation Priority

### Tier 1: Foundation (Must-Have)
1. ✅ Basic `gettext` function
2. ✅ Locale management (set, get)
3. ✅ Translation data loading (JSON)
4. ✅ String interpolation (named placeholders)

### Tier 2: Advanced Translation (Should-Have)
1. ⬜ Plural form support (`ngettext`)
2. ⬜ Context-aware translation (`pgettext`)
3. ⬜ Locale fallback
4. ⬜ Signal-based reactive translation

### Tier 3: Localization (Nice-to-Have)
1. ⬜ Date/time formatting (CLDR compliant)
2. ⬜ Number formatting
3. ⬜ Currency formatting

### Tier 4: Developer Experience (Optional)
1. ⬜ Macro-based API (`t!("key")`)
2. ⬜ Component-based API (`<Trans key="..." />`)
3. ⬜ Build-time translation key validation
4. ⬜ Missing translation warnings

---

## Reference Implementations

### Django i18n
- `gettext()`, `ngettext()`, `pgettext()`
- `{% trans %}`, `{% blocktrans %}` template tags
- `.po`/`.mo` file format

### Fluent (Mozilla)
- Message syntax: `hello = Hello, {$name}!`
- Plurals: `emails = { $count -> [one] {$count} email *[other] {$count} emails }`
- Bundle-based loading

### react-i18next
- `useTranslation()` hook
- `<Trans>` component
- Namespace support

---

## Security Considerations

### 1. XSS Protection
Translation strings are not user input, but placeholder values need escaping:

```rust
let username = user_input; // Untrusted input
let message = _("Hello, {name}!").format([("name", escape_html(username))]);
```

### 2. Injection Attacks
Never use user input as translation keys:

```rust
// ❌ Dangerous
let key = format!("message.{}", user_input);
let message = _(key);

// ✅ Safe
match user_input {
	"greeting" => _("message.greeting"),
	"farewell" => _("message.farewell"),
	_ => _("message.default"),
}
```

---

## Performance Considerations

### 1. Lazy Loading
```rust
// Initial load minimal
load_translations(Locale::new("en-US"), ["common", "auth"]).await;

// Load additional when needed
load_translations_namespace(Locale::new("en-US"), "admin").await;
```

### 2. Caching
```rust
// Memoize translation results
let message = Memo::new(move || {
	let locale = use_locale();
	_("expensive_translation")
});
```

### 3. Build-Time Optimization
```rust
// Remove unused translations
#[cfg(feature = "optimize-translations")]
fn tree_shake_translations() {
	// ...
}
```

---

## Future Extensions

### 1. RTL Language Support
- Right-to-left languages (Arabic, Hebrew, etc.)
- Auto-set `dir="rtl"` attribute

### 2. Translation Management Tools
- Web-based translation editor
- Translation progress visualization
- Translator comments

### 3. Machine Translation Integration
- Google Translate API
- DeepL API
- Auto-generate translation candidates

---

## Summary

This document serves as a guideline for i18n implementation in `reinhardt-pages`. It is recommended to reference this document during implementation and add features incrementally.
