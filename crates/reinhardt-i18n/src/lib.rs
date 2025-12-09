//! Internationalization (i18n) support for Reinhardt
//!
//! This crate provides Django-style internationalization features including:
//! - Message translation with gettext-style API
//! - Plural forms support
//! - Context-aware translations
//! - Lazy translation evaluation
//! - Message catalog management
//!
//! # Example
//! ```
//! use reinhardt_i18n::{activate, load_catalog, gettext, MessageCatalog};
//!
//! // Set up a catalog with translations
//! let mut catalog = MessageCatalog::new("ja");
//! catalog.add_translation("Hello", "こんにちは");
//!
//! // Load and activate the Japanese locale
//! load_catalog("ja", catalog).unwrap();
//! activate("ja").unwrap();
//!
//! // Translate messages
//! let greeting = gettext("Hello");
//! assert_eq!(greeting, "こんにちは");
//! ```

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Error types for i18n operations
#[derive(Debug, thiserror::Error)]
pub enum I18nError {
	#[error("Invalid locale format: {0}")]
	InvalidLocale(String),
	#[error("Catalog not found for locale: {0}")]
	CatalogNotFound(String),
	#[error("Failed to load catalog: {0}")]
	LoadError(String),
}

mod catalog;
mod lazy;
mod locale;
pub mod po_parser;
mod translation;
pub mod utils;

pub use catalog::MessageCatalog;
pub use lazy::LazyString;
pub use locale::{activate, activate_with_catalog, deactivate, get_locale};
pub use translation::{gettext, gettext_lazy, ngettext, ngettext_lazy, npgettext, pgettext};

// Re-export get_locale as get_language for compatibility
pub use locale::get_locale as get_language;

/// Catalog loader for loading message catalogs from files or other sources
pub struct CatalogLoader {
	base_path: std::path::PathBuf,
}

impl CatalogLoader {
	/// Create a new catalog loader with the given base path
	///
	/// # Example
	/// ```
	/// use reinhardt_i18n::CatalogLoader;
	///
	/// let loader = CatalogLoader::new("locale");
	/// ```
	pub fn new<P: Into<std::path::PathBuf>>(base_path: P) -> Self {
		Self {
			base_path: base_path.into(),
		}
	}

	/// Load a catalog for the given locale from a .po file
	///
	/// This method looks for .po files in the following locations:
	/// - `{base_path}/{locale}/LC_MESSAGES/django.po`
	/// - `{base_path}/{locale}/LC_MESSAGES/messages.po`
	///
	/// # Example
	/// ```no_run
	/// use reinhardt_i18n::CatalogLoader;
	///
	/// let loader = CatalogLoader::new("locale");
	/// let catalog = loader.load("fr").unwrap();
	/// ```
	pub fn load(&self, locale: &str) -> Result<MessageCatalog, String> {
		// Try multiple common .po file locations
		let possible_paths = vec![
			self.base_path
				.join(locale)
				.join("LC_MESSAGES")
				.join("django.po"),
			self.base_path
				.join(locale)
				.join("LC_MESSAGES")
				.join("messages.po"),
		];

		for path in possible_paths {
			if path.exists() {
				let file = std::fs::File::open(&path)
					.map_err(|e| format!("Failed to open .po file at {:?}: {}", path, e))?;

				return po_parser::parse_po_file(file, locale)
					.map_err(|e| format!("Failed to parse .po file: {}", e));
			}
		}

		// If no .po file found, return an empty catalog
		Ok(MessageCatalog::new(locale))
	}

	/// Load a catalog from a specific .po file path
	///
	/// # Example
	/// ```no_run
	/// use reinhardt_i18n::CatalogLoader;
	///
	/// let loader = CatalogLoader::new("locale");
	/// let catalog = loader.load_from_file("locale/fr/custom.po", "fr").unwrap();
	/// ```
	pub fn load_from_file<P: AsRef<std::path::Path>>(
		&self,
		path: P,
		locale: &str,
	) -> Result<MessageCatalog, String> {
		let file = std::fs::File::open(path.as_ref())
			.map_err(|e| format!("Failed to open .po file: {}", e))?;

		po_parser::parse_po_file(file, locale)
			.map_err(|e| format!("Failed to parse .po file: {}", e))
	}
}

/// Load a message catalog for the given locale
///
/// This function registers a message catalog with the translation system.
///
/// # Example
/// ```
/// use reinhardt_i18n::{load_catalog, MessageCatalog};
///
/// let mut catalog = MessageCatalog::new("fr");
/// catalog.add_translation("Hello", "Bonjour");
/// load_catalog("fr", catalog).unwrap();
/// ```
pub fn load_catalog(locale: &str, catalog: MessageCatalog) -> Result<(), String> {
	let mut state = TRANSLATION_STATE.write().unwrap();
	state.add_catalog(locale.to_string(), catalog);
	Ok(())
}

/// Global translation state
static TRANSLATION_STATE: Lazy<RwLock<TranslationState>> = Lazy::new(|| {
	RwLock::new(TranslationState {
		current_locale: String::new(),
		fallback_locale: String::new(),
		catalogs: HashMap::new(),
	})
});

/// Internal translation state
struct TranslationState {
	current_locale: String,
	fallback_locale: String,
	catalogs: HashMap<String, MessageCatalog>,
}

impl TranslationState {
	fn get_locale(&self) -> &str {
		if self.current_locale.is_empty() {
			"en-US"
		} else {
			&self.current_locale
		}
	}

	fn get_fallback_locale(&self) -> &str {
		if self.fallback_locale.is_empty() {
			"en-US"
		} else {
			&self.fallback_locale
		}
	}

	fn get_catalog(&self, locale: &str) -> Option<&MessageCatalog> {
		self.catalogs.get(locale)
	}

	fn set_locale(&mut self, locale: String) {
		self.current_locale = locale;
	}

	fn add_catalog(&mut self, locale: String, catalog: MessageCatalog) {
		self.catalogs.insert(locale, catalog);
	}
}
