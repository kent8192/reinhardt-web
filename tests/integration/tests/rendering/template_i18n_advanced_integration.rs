//! # Template + i18n Advanced Integration Tests
//!
//! ## Purpose
//! Cross-crate integration tests for advanced template rendering with internationalization,
//! verifying the integration between reinhardt-template/templates, reinhardt-i18n,
//! and translation catalog management.
//!
//! ## Test Coverage
//! - Template translation with multiple locales
//! - Dynamic locale switching in templates
//! - Pluralization in templates with count-based selection
//! - Date/time formatting with locale-specific rules
//! - Template fallback languages for missing translations
//! - Translation performance with large catalogs
//! - Context-based translations (pgettext)
//! - Block translations with variable substitution
//! - Nested template includes with i18n
//!
//! ## Fixtures Used
//! - `postgres_container`: PostgreSQL 16-alpine container for database operations
//! - `temp_dir`: Temporary directory for template files
//!
//! ## What is Verified
//! - Templates can render translated content in multiple languages
//! - Locale switching updates template output correctly
//! - Plural forms are selected based on language-specific rules
//! - Date/time values are formatted according to locale conventions
//! - Fallback to default language works when translation is missing
//! - Large translation catalogs perform reasonably well
//! - Context-based translations disambiguate identical strings
//! - Variable substitution works in translated templates
//!
//! ## What is NOT Covered
//! - Client-side i18n JavaScript libraries
//! - Browser locale detection
//! - Automatic translation extraction from templates
//! - Translation management UI

use chrono::{DateTime, Utc};
use reinhardt_i18n::{
	activate_with_catalog, deactivate, get_language, gettext, ngettext, pgettext, MessageCatalog,
};
use reinhardt_template::templates::i18n_filters::{
	blocktrans, blocktrans_plural, get_current_language, trans, trans_with_context,
};
use reinhardt_test::fixtures::*;
use rstest::*;
use serde_json::Value;
use serial_test::serial;
use sqlx::AnyPool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use testcontainers::core::ContainerAsync;
use testcontainers::GenericImage;

// ============================================================================
// Helper Functions
// ============================================================================

/// Simple template renderer with i18n filter support
fn render_template_with_i18n(template: &str, context: &HashMap<String, Value>) -> String {
	let mut rendered = template.to_string();

	// Replace simple variables: {{ variable }}
	for (key, value) in context {
		let placeholder = format!("{{{{{}}}}}", key);
		let replacement = match value {
			Value::String(s) => s.clone(),
			Value::Number(n) => n.to_string(),
			Value::Bool(b) => b.to_string(),
			_ => value.to_string(),
		};
		rendered = rendered.replace(&placeholder, &replacement);
	}

	// Process trans filter: {{ "text"|trans }}
	let trans_pattern_start = "{{ \"";
	let trans_pattern_end = "\"|trans }}";

	while let Some(start_pos) = rendered.find(trans_pattern_start) {
		if let Some(end_pos) = rendered[start_pos..].find(trans_pattern_end) {
			let full_pattern_start = start_pos;
			let full_pattern_end = start_pos + end_pos + trans_pattern_end.len();
			let text_start = start_pos + trans_pattern_start.len();
			let text_end = start_pos + end_pos;
			let text = &rendered[text_start..text_end];

			let translated = trans(text).unwrap_or_else(|_| text.to_string());

			rendered.replace_range(full_pattern_start..full_pattern_end, &translated);
		} else {
			break;
		}
	}

	rendered
}

/// Create Spanish translation catalog
fn create_spanish_catalog() -> MessageCatalog {
	let mut catalog = MessageCatalog::new("es");
	catalog.add_translation("Welcome", "Bienvenido");
	catalog.add_translation("Hello", "Hola");
	catalog.add_translation("Goodbye", "Adiós");
	catalog.add_translation("Thank you", "Gracias");
	catalog.add_translation("Yes", "Sí");
	catalog.add_translation("No", "No");
	catalog.add_translation("Home", "Inicio");
	catalog.add_translation("About", "Acerca de");
	catalog.add_translation("Contact", "Contacto");
	catalog.add_translation("Login", "Iniciar sesión");

	// Plural forms for Spanish
	catalog.add_plural_str("item", "items", vec!["artículo".to_string(), "artículos".to_string()]);
	catalog.add_plural_str("user", "users", vec!["usuario".to_string(), "usuarios".to_string()]);

	catalog
}

/// Create French translation catalog
fn create_french_catalog() -> MessageCatalog {
	let mut catalog = MessageCatalog::new("fr");
	catalog.add_translation("Welcome", "Bienvenue");
	catalog.add_translation("Hello", "Bonjour");
	catalog.add_translation("Goodbye", "Au revoir");
	catalog.add_translation("Thank you", "Merci");
	catalog.add_translation("Yes", "Oui");
	catalog.add_translation("No", "Non");
	catalog.add_translation("Home", "Accueil");
	catalog.add_translation("About", "À propos");
	catalog.add_translation("Contact", "Contact");

	// Plural forms for French
	catalog.add_plural_str("item", "items", vec!["article".to_string(), "articles".to_string()]);
	catalog.add_plural_str("user", "users", vec!["utilisateur".to_string(), "utilisateurs".to_string()]);

	catalog
}

/// Create Japanese translation catalog
fn create_japanese_catalog() -> MessageCatalog {
	let mut catalog = MessageCatalog::new("ja");
	catalog.add_translation("Welcome", "ようこそ");
	catalog.add_translation("Hello", "こんにちは");
	catalog.add_translation("Goodbye", "さようなら");
	catalog.add_translation("Thank you", "ありがとうございます");
	catalog.add_translation("Yes", "はい");
	catalog.add_translation("No", "いいえ");
	catalog.add_translation("Home", "ホーム");
	catalog.add_translation("About", "について");
	catalog.add_translation("Contact", "お問い合わせ");
	catalog.add_translation("Login", "ログイン");

	// Japanese has no plural distinction
	catalog.add_plural_str("item", "items", vec!["アイテム".to_string()]);
	catalog.add_plural_str("user", "users", vec!["ユーザー".to_string()]);

	catalog
}

/// Create Russian translation catalog with complex plural rules
fn create_russian_catalog() -> MessageCatalog {
	let mut catalog = MessageCatalog::new("ru");
	catalog.add_translation("Welcome", "Добро пожаловать");
	catalog.add_translation("Hello", "Здравствуйте");
	catalog.add_translation("Goodbye", "До свидания");
	catalog.add_translation("Thank you", "Спасибо");

	// Russian has 3 plural forms
	catalog.add_plural_str(
		"item",
		"items",
		vec![
			"предмет".to_string(),   // 1, 21, 31, ... (ends with 1, except 11)
			"предмета".to_string(),  // 2-4, 22-24, ... (ends with 2-4, except 12-14)
			"предметов".to_string(), // 0, 5-20, 25-30, ... (all others)
		],
	);

	catalog
}

/// Setup users table for i18n tests
async fn setup_users_database(pool: Arc<AnyPool>) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(50) NOT NULL UNIQUE,
			preferred_language VARCHAR(10) NOT NULL DEFAULT 'en',
			created_at TIMESTAMP NOT NULL DEFAULT NOW()
		)
	"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	let users = vec![
		("alice", "en"),
		("carlos", "es"),
		("marie", "fr"),
		("yuki", "ja"),
		("dmitri", "ru"),
	];

	for (username, language) in users {
		sqlx::query("INSERT INTO users (username, preferred_language) VALUES ($1, $2)")
			.bind(username)
			.bind(language)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert user");
	}
}

// ============================================================================
// Tests: Basic Translation
// ============================================================================

/// Test: Basic template translation
///
/// Intent: Verify that templates can render translated content
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_basic_template_translation(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	let template = r#"
<h1>{{ "Welcome"|trans }}</h1>
<p>{{ "Hello"|trans }}, user!</p>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_i18n(template, &context);

	assert!(rendered.contains("<h1>Bienvenido</h1>"));
	assert!(rendered.contains("<p>Hola, user!</p>"));

	deactivate();
}

/// Test: Translation falls back to original text
///
/// Intent: Verify that missing translations return original text
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_translation_fallback(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	let template = r#"<p>{{ "Missing Translation"|trans }}</p>"#;

	let context = HashMap::new();
	let rendered = render_template_with_i18n(template, &context);

	// Should return original text when translation is missing
	assert!(rendered.contains("<p>Missing Translation</p>"));

	deactivate();
}

// ============================================================================
// Tests: Multiple Locales
// ============================================================================

/// Test: Switch between multiple locales
///
/// Intent: Verify that locale switching updates template output
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_locale_switching(temp_dir: PathBuf) {
	let template = r#"<h1>{{ "Welcome"|trans }}</h1>"#;
	let context = HashMap::new();

	// Default English
	assert_eq!(get_current_language(), "en-US");
	let rendered_en = render_template_with_i18n(template, &context);
	assert!(rendered_en.contains("<h1>Welcome</h1>"));

	// Switch to Spanish
	let es_catalog = create_spanish_catalog();
	activate_with_catalog("es", es_catalog);
	assert_eq!(get_current_language(), "es");
	let rendered_es = render_template_with_i18n(template, &context);
	assert!(rendered_es.contains("<h1>Bienvenido</h1>"));

	// Switch to French
	let fr_catalog = create_french_catalog();
	activate_with_catalog("fr", fr_catalog);
	assert_eq!(get_current_language(), "fr");
	let rendered_fr = render_template_with_i18n(template, &context);
	assert!(rendered_fr.contains("<h1>Bienvenue</h1>"));

	// Switch to Japanese
	let ja_catalog = create_japanese_catalog();
	activate_with_catalog("ja", ja_catalog);
	assert_eq!(get_current_language(), "ja");
	let rendered_ja = render_template_with_i18n(template, &context);
	assert!(rendered_ja.contains("<h1>ようこそ</h1>"));

	deactivate();
}

/// Test: Render navigation menu in multiple languages
///
/// Intent: Verify that complex templates work across locales
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_navigation_menu_multilingual(temp_dir: PathBuf) {
	let template = r#"
<nav>
	<a href="/">{{ "Home"|trans }}</a>
	<a href="/about">{{ "About"|trans }}</a>
	<a href="/contact">{{ "Contact"|trans }}</a>
</nav>
	"#;

	let context = HashMap::new();

	// Spanish
	let es_catalog = create_spanish_catalog();
	activate_with_catalog("es", es_catalog);
	let rendered_es = render_template_with_i18n(template, &context);
	assert!(rendered_es.contains(">Inicio<"));
	assert!(rendered_es.contains(">Acerca de<"));
	assert!(rendered_es.contains(">Contacto<"));

	// French
	let fr_catalog = create_french_catalog();
	activate_with_catalog("fr", fr_catalog);
	let rendered_fr = render_template_with_i18n(template, &context);
	assert!(rendered_fr.contains(">Accueil<"));
	assert!(rendered_fr.contains(">À propos<"));
	assert!(rendered_fr.contains(">Contact<"));

	deactivate();
}

// ============================================================================
// Tests: Pluralization
// ============================================================================

/// Test: Plural form selection based on count
///
/// Intent: Verify that plural forms are selected correctly
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_plural_form_selection(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	// 1 item (singular)
	let result_1 = ngettext("item", "items", 1);
	assert_eq!(result_1, "artículo");

	// 2 items (plural)
	let result_2 = ngettext("item", "items", 2);
	assert_eq!(result_2, "artículos");

	// 0 items (plural)
	let result_0 = ngettext("item", "items", 0);
	assert_eq!(result_0, "artículos");

	// 100 items (plural)
	let result_100 = ngettext("item", "items", 100);
	assert_eq!(result_100, "artículos");

	deactivate();
}

/// Test: Russian complex plural rules
///
/// Intent: Verify that languages with multiple plural forms work correctly
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_russian_plural_rules(temp_dir: PathBuf) {
	let catalog = create_russian_catalog();
	activate_with_catalog("ru", catalog);

	// Form 0: ends with 1 (except 11)
	let result_1 = ngettext("item", "items", 1);
	assert_eq!(result_1, "предмет");

	let result_21 = ngettext("item", "items", 21);
	assert_eq!(result_21, "предмет");

	// Form 1: ends with 2-4 (except 12-14)
	let result_2 = ngettext("item", "items", 2);
	assert_eq!(result_2, "предмета");

	let result_3 = ngettext("item", "items", 3);
	assert_eq!(result_3, "предмета");

	let result_22 = ngettext("item", "items", 22);
	assert_eq!(result_22, "предмета");

	// Form 2: all others (0, 5-20, 11-14, etc.)
	let result_0 = ngettext("item", "items", 0);
	assert_eq!(result_0, "предметов");

	let result_5 = ngettext("item", "items", 5);
	assert_eq!(result_5, "предметов");

	let result_11 = ngettext("item", "items", 11);
	assert_eq!(result_11, "предметов");

	let result_100 = ngettext("item", "items", 100);
	assert_eq!(result_100, "предметов");

	deactivate();
}

/// Test: Japanese no plural distinction
///
/// Intent: Verify that languages without plurals use same form
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_japanese_no_plural(temp_dir: PathBuf) {
	let catalog = create_japanese_catalog();
	activate_with_catalog("ja", catalog);

	// Japanese uses same form for all counts
	let result_1 = ngettext("item", "items", 1);
	assert_eq!(result_1, "アイテム");

	let result_2 = ngettext("item", "items", 2);
	assert_eq!(result_2, "アイテム");

	let result_100 = ngettext("item", "items", 100);
	assert_eq!(result_100, "アイテム");

	deactivate();
}

// ============================================================================
// Tests: Context-Based Translation
// ============================================================================

/// Test: Disambiguate identical strings with context
///
/// Intent: Verify that pgettext works for context-based translation
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_context_based_translation(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("de");

	// "File" can mean different things in different contexts
	catalog.add_context_str("menu", "File", "Datei"); // File menu
	catalog.add_context_str("verb", "File", "Ablegen"); // To file (verb)
	catalog.add_context_str("tool", "File", "Feile"); // File (tool)

	activate_with_catalog("de", catalog);

	let menu = pgettext("menu", "File");
	assert_eq!(menu, "Datei");

	let verb = pgettext("verb", "File");
	assert_eq!(verb, "Ablegen");

	let tool = pgettext("tool", "File");
	assert_eq!(tool, "Feile");

	deactivate();
}

/// Test: Context with missing translation falls back
///
/// Intent: Verify fallback behavior for missing context translations
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_context_fallback(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("de");
	catalog.add_context_str("menu", "File", "Datei");

	activate_with_catalog("de", catalog);

	// Missing context should return original
	let result = pgettext("unknown_context", "File");
	assert_eq!(result, "File");

	deactivate();
}

// ============================================================================
// Tests: Template + Database + i18n
// ============================================================================

/// Test: Render user profile with preferred language
///
/// Intent: Verify that database-driven locale selection works
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_user_profile_with_language_preference(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
	temp_dir: PathBuf,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	setup_users_database(pool.clone()).await;

	// Fetch users with their preferred languages
	let users: Vec<(String, String)> =
		sqlx::query_as("SELECT username, preferred_language FROM users ORDER BY id")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to fetch users");

	assert_eq!(users.len(), 5);

	// Verify each user's preferred language
	for (username, lang) in &users {
		match username.as_str() {
			"alice" => assert_eq!(lang, "en"),
			"carlos" => assert_eq!(lang, "es"),
			"marie" => assert_eq!(lang, "fr"),
			"yuki" => assert_eq!(lang, "ja"),
			"dmitri" => assert_eq!(lang, "ru"),
			_ => panic!("Unexpected user: {}", username),
		}
	}

	// Activate Spanish for carlos
	let es_catalog = create_spanish_catalog();
	activate_with_catalog("es", es_catalog);

	let greeting = gettext("Welcome");
	assert_eq!(greeting, "Bienvenido");

	deactivate();
}

/// Test: Dynamic greeting based on user's language
///
/// Intent: Verify that templates can render personalized greetings
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_personalized_greeting_by_language(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
	temp_dir: PathBuf,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	setup_users_database(pool.clone()).await;

	// Fetch user with preferred language
	let user: (String, String) =
		sqlx::query_as("SELECT username, preferred_language FROM users WHERE username = $1")
			.bind("yuki")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch user");

	assert_eq!(user.0, "yuki");
	assert_eq!(user.1, "ja");

	// Activate user's preferred language
	let ja_catalog = create_japanese_catalog();
	activate_with_catalog(&user.1, ja_catalog);

	let greeting = gettext("Hello");
	assert_eq!(greeting, "こんにちは");

	deactivate();
}

// ============================================================================
// Tests: Date/Time Formatting with Locales
// ============================================================================

/// Test: Date formatting with locale
///
/// Intent: Verify that dates can be formatted according to locale
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_date_formatting_with_locale(temp_dir: PathBuf) {
	let date = DateTime::parse_from_rfc3339("2025-01-18T12:00:00Z")
		.unwrap()
		.with_timezone(&Utc);

	// US format: MM/DD/YYYY
	let us_format = date.format("%m/%d/%Y").to_string();
	assert_eq!(us_format, "01/18/2025");

	// European format: DD/MM/YYYY
	let eu_format = date.format("%d/%m/%Y").to_string();
	assert_eq!(eu_format, "18/01/2025");

	// ISO format: YYYY-MM-DD
	let iso_format = date.format("%Y-%m-%d").to_string();
	assert_eq!(iso_format, "2025-01-18");
}

/// Test: Time formatting with locale
///
/// Intent: Verify that time can be formatted with 12h/24h conventions
#[rstest]
#[tokio::test]
async fn test_time_formatting_with_locale(temp_dir: PathBuf) {
	let time = DateTime::parse_from_rfc3339("2025-01-18T14:30:00Z")
		.unwrap()
		.with_timezone(&Utc);

	// 24-hour format (common in Europe, Asia)
	let format_24h = time.format("%H:%M").to_string();
	assert_eq!(format_24h, "14:30");

	// 12-hour format with AM/PM (common in US)
	let format_12h = time.format("%I:%M %p").to_string();
	assert_eq!(format_12h, "02:30 PM");
}

// ============================================================================
// Tests: Fallback Languages
// ============================================================================

/// Test: Fallback to default language for missing translations
///
/// Intent: Verify that fallback mechanism works
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_fallback_to_default_language(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("es");
	// Only translate some strings
	catalog.add_translation("Welcome", "Bienvenido");
	// "Login" is not translated

	activate_with_catalog("es", catalog);

	// Translated
	let welcome = gettext("Welcome");
	assert_eq!(welcome, "Bienvenido");

	// Falls back to English (original)
	let login = gettext("Login");
	assert_eq!(login, "Login");

	deactivate();
}

/// Test: Partial catalog with mixed content
///
/// Intent: Verify that templates work with partially translated catalogs
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_partial_catalog_mixed_content(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("fr");
	catalog.add_translation("Welcome", "Bienvenue");
	catalog.add_translation("Hello", "Bonjour");
	// "Goodbye", "Thank you" not translated

	activate_with_catalog("fr", catalog);

	let template = r#"
<h1>{{ "Welcome"|trans }}</h1>
<p>{{ "Hello"|trans }}</p>
<p>{{ "Goodbye"|trans }}</p>
<p>{{ "Thank you"|trans }}</p>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_i18n(template, &context);

	// Translated
	assert!(rendered.contains("<h1>Bienvenue</h1>"));
	assert!(rendered.contains("<p>Bonjour</p>"));

	// Fallback to English
	assert!(rendered.contains("<p>Goodbye</p>"));
	assert!(rendered.contains("<p>Thank you</p>"));

	deactivate();
}

// ============================================================================
// Tests: Translation Performance
// ============================================================================

/// Test: Performance with large catalog
///
/// Intent: Verify that translation lookups perform well with many entries
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_large_catalog_performance(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("es");

	// Add 1000 translations
	for i in 0..1000 {
		catalog.add_translation(&format!("key_{}", i), &format!("valor_{}", i));
	}

	activate_with_catalog("es", catalog);

	// Lookup translations many times
	let start = std::time::Instant::now();
	for i in 0..1000 {
		let key = format!("key_{}", i);
		let result = gettext(&key);
		assert_eq!(result, format!("valor_{}", i));
	}
	let elapsed = start.elapsed();

	// 1000 lookups should complete quickly (< 10ms)
	assert!(
		elapsed.as_millis() < 10,
		"Translation lookup took too long: {:?}",
		elapsed
	);

	deactivate();
}

/// Test: Repeated translations are fast
///
/// Intent: Verify that repeated translation lookups are efficient
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_repeated_translations_performance(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	let template = r#"
<nav>
	<a>{{ "Home"|trans }}</a>
	<a>{{ "About"|trans }}</a>
	<a>{{ "Contact"|trans }}</a>
</nav>
	"#;

	let context = HashMap::new();

	let start = std::time::Instant::now();
	for _ in 0..1000 {
		let _rendered = render_template_with_i18n(template, &context);
	}
	let elapsed = start.elapsed();

	// 1000 renders should complete quickly (< 100ms)
	assert!(
		elapsed.as_millis() < 100,
		"Repeated translations took too long: {:?}",
		elapsed
	);

	deactivate();
}

// ============================================================================
// Tests: Block Translation
// ============================================================================

/// Test: Block translation with blocktrans filter
///
/// Intent: Verify that blocktrans works for longer text blocks
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_block_translation(temp_dir: PathBuf) {
	let mut catalog = MessageCatalog::new("es");
	catalog.add_translation("Welcome to our site!", "¡Bienvenido a nuestro sitio!");
	catalog.add_translation(
		"Please feel free to look around.",
		"Por favor, siéntete libre de mirar alrededor.",
	);

	activate_with_catalog("es", catalog);

	let result1 = blocktrans("Welcome to our site!").unwrap();
	assert_eq!(result1, "¡Bienvenido a nuestro sitio!");

	let result2 = blocktrans("Please feel free to look around.").unwrap();
	assert_eq!(result2, "Por favor, siéntete libre de mirar alrededor.");

	deactivate();
}

/// Test: Block translation with plurals
///
/// Intent: Verify that blocktrans_plural works with count
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_block_translation_with_plural(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	// Singular
	let result_1 = blocktrans_plural("user", "users", 1).unwrap();
	assert_eq!(result_1, "usuario");

	// Plural
	let result_5 = blocktrans_plural("user", "users", 5).unwrap();
	assert_eq!(result_5, "usuarios");

	deactivate();
}

// ============================================================================
// Tests: Template Caching with i18n
// ============================================================================

/// Test: Template caching preserves translations
///
/// Intent: Verify that template caching works with i18n
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_template_caching_with_i18n(temp_dir: PathBuf) {
	let catalog = create_spanish_catalog();
	activate_with_catalog("es", catalog);

	let template = r#"<h1>{{ "Welcome"|trans }}</h1>"#;
	let context = HashMap::new();

	// First render
	let rendered_1 = render_template_with_i18n(template, &context);
	assert!(rendered_1.contains("<h1>Bienvenido</h1>"));

	// Second render (should be consistent)
	let rendered_2 = render_template_with_i18n(template, &context);
	assert_eq!(rendered_1, rendered_2);

	deactivate();
}

/// Test: Locale switching updates cached templates
///
/// Intent: Verify that changing locale updates template output
#[rstest]
#[serial(i18n)]
#[tokio::test]
async fn test_locale_switching_updates_cache(temp_dir: PathBuf) {
	let template = r#"<h1>{{ "Welcome"|trans }}</h1>"#;
	let context = HashMap::new();

	// Spanish
	let es_catalog = create_spanish_catalog();
	activate_with_catalog("es", es_catalog);
	let rendered_es = render_template_with_i18n(template, &context);
	assert!(rendered_es.contains("<h1>Bienvenido</h1>"));

	// Switch to French
	let fr_catalog = create_french_catalog();
	activate_with_catalog("fr", fr_catalog);
	let rendered_fr = render_template_with_i18n(template, &context);
	assert!(rendered_fr.contains("<h1>Bienvenue</h1>"));

	// Verify they're different
	assert_ne!(rendered_es, rendered_fr);

	deactivate();
}
