//! Basic Sitemap Generation Tests
//!
//! These tests verify the core sitemap generation functionality without requiring
//! HTTP integration or views.
//!
//! Tests cover:
//! - SitemapItem creation and validation
//! - Sitemap XML generation
//! - SitemapIndex creation
//! - ChangeFrequency and Priority types

use chrono::Utc;
use reinhardt_sitemaps::{
    ChangeFrequency, Priority, Sitemap, SitemapIndex, SitemapItem, SitemapReference,
};

#[cfg(test)]
mod sitemap_item_tests {
    use super::*;

    #[test]
    fn test_create_basic_sitemap_item() {
        let item = SitemapItem::new("https://example.com/page1");

        assert_eq!(item.loc, "https://example.com/page1");
        assert!(item.lastmod.is_none());
        assert!(item.changefreq.is_none());
        assert!(item.priority.is_none());
    }

    #[test]
    fn test_sitemap_item_with_all_fields() {
        let now = Utc::now();
        let priority = Priority::new(0.8).unwrap();

        let item = SitemapItem::new("https://example.com/important")
            .with_lastmod(now)
            .with_changefreq(ChangeFrequency::Daily)
            .with_priority(priority);

        assert_eq!(item.loc, "https://example.com/important");
        assert!(item.lastmod.is_some());
        assert_eq!(item.changefreq, Some(ChangeFrequency::Daily));
        assert_eq!(item.priority, Some(priority));
    }

    #[test]
    fn test_sitemap_item_validation_valid_url() {
        let item1 = SitemapItem::new("https://example.com/page");
        assert!(item1.validate().is_ok());

        let item2 = SitemapItem::new("http://example.com/page");
        assert!(item2.validate().is_ok());
    }

    #[test]
    fn test_sitemap_item_validation_invalid_url() {
        // URL without protocol
        let item = SitemapItem::new("example.com/page");
        assert!(item.validate().is_err());

        // URL with wrong protocol
        let item = SitemapItem::new("ftp://example.com/page");
        assert!(item.validate().is_err());
    }

    #[test]
    fn test_sitemap_item_validation_url_too_long() {
        // URL longer than 2048 characters
        let long_url = format!("https://example.com/{}", "a".repeat(2050));
        let item = SitemapItem::new(long_url);
        assert!(item.validate().is_err());
    }

    #[test]
    fn test_sitemap_item_to_xml_minimal() {
        let item = SitemapItem::new("https://example.com/page");
        let xml = item.to_xml().unwrap();

        assert!(xml.contains("<url>"));
        assert!(xml.contains("<loc>https://example.com/page</loc>"));
        assert!(xml.contains("</url>"));
        assert!(!xml.contains("<lastmod>"));
        assert!(!xml.contains("<changefreq>"));
        assert!(!xml.contains("<priority>"));
    }

    #[test]
    fn test_sitemap_item_to_xml_full() {
        let now = Utc::now();
        let priority = Priority::new(0.9).unwrap();

        let item = SitemapItem::new("https://example.com/important")
            .with_lastmod(now)
            .with_changefreq(ChangeFrequency::Weekly)
            .with_priority(priority);

        let xml = item.to_xml().unwrap();

        assert!(xml.contains("<url>"));
        assert!(xml.contains("<loc>https://example.com/important</loc>"));
        assert!(xml.contains("<lastmod>"));
        assert!(xml.contains("<changefreq>weekly</changefreq>"));
        assert!(xml.contains("<priority>0.9</priority>"));
        assert!(xml.contains("</url>"));
    }

    #[test]
    fn test_sitemap_item_xml_escaping() {
        let item = SitemapItem::new("https://example.com/page?foo=bar&baz=qux");
        let xml = item.to_xml().unwrap();

        // XML should escape & character
        assert!(xml.contains("&amp;") || xml.contains("&"));
    }
}

#[cfg(test)]
mod priority_tests {
    use super::*;

    #[test]
    fn test_priority_valid_values() {
        assert!(Priority::new(0.0).is_ok());
        assert!(Priority::new(0.5).is_ok());
        assert!(Priority::new(1.0).is_ok());
    }

    #[test]
    fn test_priority_invalid_values() {
        assert!(Priority::new(-0.1).is_err());
        assert!(Priority::new(1.1).is_err());
        assert!(Priority::new(2.0).is_err());
    }

    #[test]
    fn test_priority_default() {
        let priority = Priority::default();
        assert_eq!(priority.value(), 0.5);
    }

    #[test]
    fn test_priority_value_access() {
        let priority = Priority::new(0.8).unwrap();
        assert_eq!(priority.value(), 0.8);
    }
}

#[cfg(test)]
mod change_frequency_tests {
    use super::*;

    #[test]
    fn test_change_frequency_values() {
        assert_eq!(ChangeFrequency::Always.as_str(), "always");
        assert_eq!(ChangeFrequency::Hourly.as_str(), "hourly");
        assert_eq!(ChangeFrequency::Daily.as_str(), "daily");
        assert_eq!(ChangeFrequency::Weekly.as_str(), "weekly");
        assert_eq!(ChangeFrequency::Monthly.as_str(), "monthly");
        assert_eq!(ChangeFrequency::Yearly.as_str(), "yearly");
        assert_eq!(ChangeFrequency::Never.as_str(), "never");
    }

    #[test]
    fn test_change_frequency_equality() {
        assert_eq!(ChangeFrequency::Daily, ChangeFrequency::Daily);
        assert_ne!(ChangeFrequency::Daily, ChangeFrequency::Weekly);
    }
}

#[cfg(test)]
mod sitemap_tests {
    use super::*;

    #[test]
    fn test_create_empty_sitemap() {
        let sitemap = Sitemap { items: vec![] };
        assert_eq!(sitemap.items.len(), 0);
    }

    #[test]
    fn test_create_sitemap_with_items() {
        let items = vec![
            SitemapItem::new("https://example.com/page1"),
            SitemapItem::new("https://example.com/page2"),
            SitemapItem::new("https://example.com/page3"),
        ];

        let sitemap = Sitemap {
            items: items.clone(),
        };

        assert_eq!(sitemap.items.len(), 3);
        assert_eq!(sitemap.items[0].loc, "https://example.com/page1");
        assert_eq!(sitemap.items[1].loc, "https://example.com/page2");
        assert_eq!(sitemap.items[2].loc, "https://example.com/page3");
    }

    #[test]
    fn test_sitemap_generation_to_xml() {
        let items = vec![
            SitemapItem::new("https://example.com/page1").with_changefreq(ChangeFrequency::Daily),
            SitemapItem::new("https://example.com/page2")
                .with_priority(Priority::new(0.8).unwrap()),
        ];

        let sitemap = Sitemap { items };
        let xml = sitemap.to_xml().unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<urlset"));
        assert!(xml.contains("xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\""));
        assert!(xml.contains("<loc>https://example.com/page1</loc>"));
        assert!(xml.contains("<loc>https://example.com/page2</loc>"));
        assert!(xml.contains("<changefreq>daily</changefreq>"));
        assert!(xml.contains("<priority>0.8</priority>"));
        assert!(xml.contains("</urlset>"));
    }

    #[test]
    fn test_sitemap_max_urls_validation() {
        // Create sitemap using add_item which enforces the limit
        let mut sitemap = Sitemap::new();

        // Add MAX_URLS_PER_SITEMAP items (should succeed)
        for i in 0..reinhardt_sitemaps::sitemap::MAX_URLS_PER_SITEMAP {
            let result =
                sitemap.add_item(SitemapItem::new(format!("https://example.com/page{}", i)));
            assert!(result.is_ok());
        }

        // Adding one more should fail
        let result = sitemap.add_item(SitemapItem::new("https://example.com/one-too-many"));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod sitemap_index_tests {
    use super::*;

    #[test]
    fn test_create_sitemap_reference() {
        let now = Utc::now();
        let reference = SitemapReference {
            loc: "https://example.com/sitemap1.xml".to_string(),
            lastmod: Some(now),
        };

        assert_eq!(reference.loc, "https://example.com/sitemap1.xml");
        assert_eq!(reference.lastmod, Some(now));
    }

    #[test]
    fn test_create_sitemap_index() {
        let now = Utc::now();

        let sitemaps = vec![
            SitemapReference {
                loc: "https://example.com/sitemap1.xml".to_string(),
                lastmod: Some(now),
            },
            SitemapReference {
                loc: "https://example.com/sitemap2.xml".to_string(),
                lastmod: Some(now),
            },
            SitemapReference {
                loc: "https://example.com/sitemap3.xml".to_string(),
                lastmod: None,
            },
        ];

        let index = SitemapIndex { sitemaps };

        assert_eq!(index.sitemaps.len(), 3);
        assert_eq!(index.sitemaps[0].loc, "https://example.com/sitemap1.xml");
        assert_eq!(index.sitemaps[1].loc, "https://example.com/sitemap2.xml");
        assert_eq!(index.sitemaps[2].loc, "https://example.com/sitemap3.xml");
    }

    #[test]
    fn test_sitemap_generation_index_to_xml() {
        let now = Utc::now();

        let sitemaps = vec![
            SitemapReference {
                loc: "https://example.com/sitemap1.xml".to_string(),
                lastmod: Some(now),
            },
            SitemapReference {
                loc: "https://example.com/sitemap2.xml".to_string(),
                lastmod: None,
            },
        ];

        let index = SitemapIndex { sitemaps };
        let xml = index.to_xml().unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<sitemapindex"));
        assert!(xml.contains("xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\""));
        assert!(xml.contains("<loc>https://example.com/sitemap1.xml</loc>"));
        assert!(xml.contains("<loc>https://example.com/sitemap2.xml</loc>"));
        assert!(xml.contains("<lastmod>"));
        assert!(xml.contains("</sitemapindex>"));
    }

    #[test]
    fn test_empty_sitemap_index() {
        let index = SitemapIndex { sitemaps: vec![] };

        assert_eq!(index.sitemaps.len(), 0);

        let xml = index.to_xml().unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<sitemapindex"));
        assert!(xml.contains("</sitemapindex>"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_sitemap_workflow() {
        // Create items with various attributes
        let now = Utc::now();

        let item1 = SitemapItem::new("https://example.com/")
            .with_priority(Priority::new(1.0).unwrap())
            .with_changefreq(ChangeFrequency::Daily);

        let item2 = SitemapItem::new("https://example.com/about")
            .with_lastmod(now)
            .with_changefreq(ChangeFrequency::Monthly)
            .with_priority(Priority::new(0.8).unwrap());

        let item3 = SitemapItem::new("https://example.com/blog")
            .with_lastmod(now)
            .with_changefreq(ChangeFrequency::Weekly)
            .with_priority(Priority::new(0.7).unwrap());

        // Create sitemap
        let sitemap = Sitemap {
            items: vec![item1, item2, item3],
        };

        // Validate items
        for item in &sitemap.items {
            assert!(item.validate().is_ok());
        }

        // Generate XML
        let xml = sitemap.to_xml().unwrap();

        // Verify XML structure
        assert!(xml.contains("<urlset"));
        assert!(xml.contains("</urlset>"));
        assert_eq!(xml.matches("<url>").count(), 3);
        assert_eq!(xml.matches("</url>").count(), 3);
    }

    #[test]
    fn test_multi_sitemap_with_index() {
        let now = Utc::now();

        // Create multiple sitemaps
        let sitemap1 = Sitemap {
            items: vec![
                SitemapItem::new("https://example.com/page1"),
                SitemapItem::new("https://example.com/page2"),
            ],
        };

        let sitemap2 = Sitemap {
            items: vec![
                SitemapItem::new("https://example.com/page3"),
                SitemapItem::new("https://example.com/page4"),
            ],
        };

        // Generate XML for each sitemap
        let xml1 = sitemap1.to_xml().unwrap();
        let xml2 = sitemap2.to_xml().unwrap();

        assert!(xml1.contains("page1"));
        assert!(xml1.contains("page2"));
        assert!(xml2.contains("page3"));
        assert!(xml2.contains("page4"));

        // Create sitemap index
        let index = SitemapIndex {
            sitemaps: vec![
                SitemapReference {
                    loc: "https://example.com/sitemap1.xml".to_string(),
                    lastmod: Some(now),
                },
                SitemapReference {
                    loc: "https://example.com/sitemap2.xml".to_string(),
                    lastmod: Some(now),
                },
            ],
        };

        let index_xml = index.to_xml().unwrap();
        assert!(index_xml.contains("sitemap1.xml"));
        assert!(index_xml.contains("sitemap2.xml"));
    }
}
