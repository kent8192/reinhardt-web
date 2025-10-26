//! Message catalog for storing translations

use std::collections::HashMap;

/// A message catalog containing translations for a specific locale
///
/// # Example
/// ```
/// use reinhardt_i18n::MessageCatalog;
///
/// let mut catalog = MessageCatalog::new("fr");
/// catalog.add_translation("Hello", "Bonjour");
/// catalog.add_plural_str("item", "items", vec!["article", "articles"]);
///
/// assert_eq!(catalog.get("Hello"), Some(&"Bonjour".to_string()));
/// assert_eq!(catalog.get_plural("item", 1), Some(&"article".to_string()));
/// assert_eq!(catalog.get_plural("item", 5), Some(&"articles".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct MessageCatalog {
    locale: String,
    messages: HashMap<String, String>,
    plurals: HashMap<String, Vec<String>>,
    contexts: HashMap<(String, String), String>,
    context_plurals: HashMap<(String, String), Vec<String>>,
}

impl MessageCatalog {
    /// Create a new message catalog for the given locale
    pub fn new(locale: &str) -> Self {
        Self {
            locale: locale.to_string(),
            messages: HashMap::new(),
            plurals: HashMap::new(),
            contexts: HashMap::new(),
            context_plurals: HashMap::new(),
        }
    }

    /// Get the locale for this catalog
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Add a simple translation
    pub fn add_translation(&mut self, message: &str, translation: &str) {
        self.messages
            .insert(message.to_string(), translation.to_string());
    }

    /// Add a simple translation (alias for add_translation)
    pub fn add(&mut self, message: String, translation: String) {
        self.messages.insert(message, translation);
    }

    /// Add a plural translation with Vec<String>
    /// If the singular key contains a colon (e.g., "context:message"), it will be
    /// treated as a contextual plural and split accordingly.
    pub fn add_plural(&mut self, singular: String, forms: Vec<String>) {
        // Check if the key contains a context (format: "context:message")
        if let Some(colon_pos) = singular.find(':') {
            let context = singular[..colon_pos].to_string();
            let message = singular[colon_pos + 1..].to_string();
            self.context_plurals.insert((context, message), forms);
        } else {
            self.plurals.insert(singular, forms);
        }
    }

    /// Add a plural translation with string slices
    pub fn add_plural_str(&mut self, singular: &str, _plural: &str, forms: Vec<&str>) {
        self.plurals.insert(
            singular.to_string(),
            forms.iter().map(|s| s.to_string()).collect(),
        );
    }

    /// Add a contextual translation
    pub fn add_context(&mut self, context: String, message: String, translation: String) {
        self.contexts.insert((context, message), translation);
    }

    /// Add a contextual translation with string slices
    pub fn add_context_str(&mut self, context: &str, message: &str, translation: &str) {
        self.contexts.insert(
            (context.to_string(), message.to_string()),
            translation.to_string(),
        );
    }

    /// Add a contextual plural translation
    pub fn add_context_plural(
        &mut self,
        context: &str,
        singular: &str,
        _plural: &str,
        forms: Vec<&str>,
    ) {
        self.context_plurals.insert(
            (context.to_string(), singular.to_string()),
            forms.iter().map(|s| s.to_string()).collect(),
        );
    }

    /// Get a translation
    pub fn get(&self, message: &str) -> Option<&String> {
        self.messages.get(message)
    }

    /// Get a plural translation
    pub fn get_plural(&self, singular: &str, count: usize) -> Option<&String> {
        let forms = self.plurals.get(singular)?;
        let index = self.plural_form(count);
        forms.get(index)
    }

    /// Get a contextual translation
    pub fn get_context(&self, context: &str, message: &str) -> Option<&String> {
        self.contexts
            .get(&(context.to_string(), message.to_string()))
    }

    /// Get a contextual plural translation
    pub fn get_context_plural(
        &self,
        context: &str,
        singular: &str,
        count: usize,
    ) -> Option<&String> {
        let forms = self
            .context_plurals
            .get(&(context.to_string(), singular.to_string()))?;
        let index = self.plural_form(count);
        forms.get(index)
    }

    /// Determine the plural form index for a given count
    /// Uses language-specific plural rules based on locale
    fn plural_form(&self, count: usize) -> usize {
        // Japanese, Chinese, Korean: no plural forms (always index 0)
        if self.locale.starts_with("ja")
            || self.locale.starts_with("zh")
            || self.locale.starts_with("ko")
        {
            0
        }
        // French and similar languages: 0 and 1 are singular (index 0), 2+ are plural (index 1)
        else if self.locale.starts_with("fr") {
            if count == 0 || count == 1 { 0 } else { 1 }
        }
        // English and default: 1 is singular (index 0), 0 and 2+ are plural (index 1)
        else if count == 1 {
            0
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_catalog_basic() {
        let mut catalog = MessageCatalog::new("es");
        catalog.add_translation("Good morning", "Buenos días");

        assert_eq!(
            catalog.get("Good morning"),
            Some(&"Buenos días".to_string())
        );
        assert_eq!(catalog.get("Unknown"), None);
    }

    #[test]
    fn test_message_catalog_plural() {
        let mut catalog = MessageCatalog::new("fr");
        catalog.add_plural_str("car", "cars", vec!["voiture", "voitures"]);

        assert_eq!(catalog.get_plural("car", 1), Some(&"voiture".to_string()));
        assert_eq!(catalog.get_plural("car", 3), Some(&"voitures".to_string()));
    }

    #[test]
    fn test_message_catalog_context() {
        let mut catalog = MessageCatalog::new("de");
        catalog.add_context_str("menu", "File", "Datei");
        catalog.add_context_str("verb", "File", "Ablegen");

        assert_eq!(
            catalog.get_context("menu", "File"),
            Some(&"Datei".to_string())
        );
        assert_eq!(
            catalog.get_context("verb", "File"),
            Some(&"Ablegen".to_string())
        );
    }
}
