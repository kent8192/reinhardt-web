// Real i18n sitemap integration tests
// These tests use actual i18n functionality from reinhardt-sitemaps

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::i18n_integration::*;
use reinhardt_sitemaps::*;

// Provider that generates multilingual URLs
#[derive(Debug)]
struct I18nSitemapProvider {
    paths: Vec<String>,
    languages: Vec<Language>,
    base_url: String,
}

impl I18nSitemapProvider {
    fn new(paths: Vec<String>, languages: Vec<Language>, base_url: String) -> Self {
        Self {
            paths,
            languages,
            base_url,
        }
    }

    fn get_alternates(&self, path: &str) -> Vec<AlternateLink> {
        generate_alternate_links(path, &self.languages, &self.base_url)
    }
}

impl SitemapProvider for I18nSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();
        add_i18n_paths_to_sitemap(&mut sitemap, &self.paths, &self.languages, &self.base_url)?;
        Ok(sitemap)
    }
}

#[test]
fn test_simple_i18n_sitemap() {
    // Real i18n sitemap with multiple languages
    let provider = I18nSitemapProvider::new(
        vec!["/about".to_string(), "/contact".to_string()],
        vec![Language::En, Language::Ja, Language::Fr],
        "https://example.com".to_string(),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Verify all language versions are present
    assert!(response.content.contains("https://example.com/en/about"));
    assert!(response.content.contains("https://example.com/ja/about"));
    assert!(response.content.contains("https://example.com/fr/about"));
    assert!(response.content.contains("https://example.com/en/contact"));
    assert!(response.content.contains("https://example.com/ja/contact"));
    assert!(response.content.contains("https://example.com/fr/contact"));

    // Should have 6 URLs total (3 languages × 2 pages)
    assert_eq!(response.content.matches("<url>").count(), 6);
}

#[test]
fn test_sitemap_without_entries_i18n() {
    // Empty i18n sitemap should render without errors
    let provider = I18nSitemapProvider::new(
        vec![],
        vec![Language::En, Language::Ja],
        "https://example.com".to_string(),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should have valid XML structure with no URLs
    assert!(response.content.contains("<urlset"));
    assert!(!response.content.contains("<url>"));
}

#[test]
fn test_sitemap_language_filtering() {
    // Create sitemap with multiple languages
    let mut sitemap = Sitemap::new();
    add_i18n_paths_to_sitemap(
        &mut sitemap,
        &vec!["/page1".to_string(), "/page2".to_string()],
        &vec![Language::En, Language::Ja, Language::Fr],
        "https://example.com",
    )
    .unwrap();

    // Filter to only Japanese
    let filtered = filter_by_language(sitemap, Language::Ja);

    // Should only have Japanese URLs
    assert_eq!(filtered.items.len(), 2);
    assert!(filtered.items[0].loc.contains("/ja/"));
    assert!(filtered.items[1].loc.contains("/ja/"));

    // Verify with view
    #[derive(Debug)]
    struct StaticProvider(Sitemap);
    impl SitemapProvider for StaticProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(self.0.clone())
        }
    }

    let view = SitemapView::new(Box::new(StaticProvider(filtered)));
    let response = view.render().unwrap();

    assert!(response.content.contains("https://example.com/ja/page1"));
    assert!(response.content.contains("https://example.com/ja/page2"));
    assert!(!response.content.contains("/en/"));
    assert!(!response.content.contains("/fr/"));
}

#[test]
fn test_alternate_links() {
    // Test hreflang alternate links generation
    let provider = I18nSitemapProvider::new(
        vec!["/article".to_string()],
        vec![Language::En, Language::Ja, Language::Fr],
        "https://example.com".to_string(),
    );

    let alternates = provider.get_alternates("/article");

    assert_eq!(alternates.len(), 3);
    assert_eq!(alternates[0].lang, Language::En);
    assert_eq!(alternates[0].url, "https://example.com/en/article");
    assert_eq!(alternates[1].lang, Language::Ja);
    assert_eq!(alternates[1].url, "https://example.com/ja/article");
    assert_eq!(alternates[2].lang, Language::Fr);
    assert_eq!(alternates[2].url, "https://example.com/fr/article");
}

#[test]
fn test_default_language() {
    // Test with English as default language
    #[derive(Debug)]
    struct DefaultLanguageProvider {
        paths: Vec<String>,
        default_lang: Language,
        base_url: String,
    }

    impl SitemapProvider for DefaultLanguageProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            add_i18n_paths_to_sitemap(
                &mut sitemap,
                &self.paths,
                &vec![self.default_lang],
                &self.base_url,
            )?;
            Ok(sitemap)
        }
    }

    let provider = DefaultLanguageProvider {
        paths: vec!["/home".to_string()],
        default_lang: Language::En,
        base_url: "https://example.com".to_string(),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Default language URL should be present
    assert!(response.content.contains("https://example.com/en/home"));
}

#[test]
fn test_i18n_sitemap_with_lastmod() {
    // i18n sitemap with lastmod - all language versions share same lastmod
    #[derive(Debug)]
    struct I18nLastmodProvider {
        paths: Vec<(String, DateTime<Utc>)>,
        languages: Vec<Language>,
        base_url: String,
    }

    impl SitemapProvider for I18nLastmodProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for (path, lastmod) in &self.paths {
                let items = generate_i18n_sitemap_items(
                    path,
                    &self.languages,
                    &self.base_url,
                    Some(*lastmod),
                    None,
                    None,
                )?;
                for item in items {
                    sitemap.add_item(item)?;
                }
            }
            Ok(sitemap)
        }

        fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
            self.paths.iter().map(|(_, lastmod)| *lastmod).max()
        }
    }

    let now = Utc::now();
    let provider = I18nLastmodProvider {
        paths: vec![("/page1".to_string(), now)],
        languages: vec![Language::En, Language::Ja],
        base_url: "https://example.com".to_string(),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Both language versions should have lastmod
    assert!(response.content.contains("<lastmod>"));
    // Should have Last-Modified header
    assert!(response.has_header("Last-Modified"));
}

#[test]
fn test_i18n_paginated_sitemap() {
    // Large i18n sitemap with pagination
    #[derive(Debug)]
    struct PaginatedI18nProvider {
        items_per_language: usize,
        languages: Vec<Language>,
    }

    impl SitemapProvider for PaginatedI18nProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            self.get_paginated_sitemap(1)?
                .ok_or_else(|| SitemapError::Generation("No sitemap available".to_string()))
        }

        fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
            if page > 2 {
                return Ok(None);
            }

            let mut sitemap = Sitemap::new();
            let start = (page - 1) * 10;
            let end = start + 10;

            for i in start..end.min(self.items_per_language) {
                for lang in &self.languages {
                    let url = format!("https://example.com/{}/page{}", lang.code(), i);
                    sitemap.add_item(SitemapItem::new(url))?;
                }
            }

            Ok(Some(sitemap))
        }

        fn get_page_count(&self) -> usize {
            2
        }
    }

    let provider = PaginatedI18nProvider {
        items_per_language: 15,
        languages: vec![Language::En, Language::Ja],
    };

    // Page 1 should have 20 URLs (10 items × 2 languages)
    let view1 = SitemapView::new(Box::new(provider)).with_page(1);
    let response1 = view1.render().unwrap();
    assert_eq!(response1.content.matches("<url>").count(), 20);

    // Page 2 should have 10 URLs (5 remaining items × 2 languages)
    let provider2 = PaginatedI18nProvider {
        items_per_language: 15,
        languages: vec![Language::En, Language::Ja],
    };
    let view2 = SitemapView::new(Box::new(provider2)).with_page(2);
    let response2 = view2.render().unwrap();
    assert_eq!(response2.content.matches("<url>").count(), 10);
}

#[test]
fn test_language_code_conversion() {
    // Test Language enum conversions
    assert_eq!(Language::En.code(), "en");
    assert_eq!(Language::Ja.code(), "ja");
    assert_eq!(Language::Fr.code(), "fr");

    assert_eq!(Language::from_code("en"), Some(Language::En));
    assert_eq!(Language::from_code("ja"), Some(Language::Ja));
    assert_eq!(Language::from_code("invalid"), None);
}

#[test]
fn test_generate_i18n_urls() {
    // Test URL generation helper
    let urls = generate_i18n_urls(
        "/about",
        &[Language::En, Language::Ja, Language::Fr],
        "https://example.com",
    );

    assert_eq!(urls.len(), 3);
    assert_eq!(urls[0], "https://example.com/en/about");
    assert_eq!(urls[1], "https://example.com/ja/about");
    assert_eq!(urls[2], "https://example.com/fr/about");
}
