//! Integration tests for flatpages with other reinhardt crates
//!
//! **REQUIRES DATABASE**: These integration tests require a running PostgreSQL database.
//!
//! ## Automatic Setup (testcontainers)
//!
//! The flatpages_common module includes testcontainers support for automatic PostgreSQL setup.
//! However, this is disabled by default in Cargo.toml due to long container startup times.
//!
//! ## Manual Setup (Recommended for CI)
//!
//! To run these tests with a manual PostgreSQL instance:
//!
//! ```bash
//! # Start PostgreSQL container
//! docker run --rm -d -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres:17-alpine
//!
//! # Run tests with manual database
//! TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres \
//!     cargo test --package reinhardt-integration-tests --test flatpages_integration_tests
//! ```
//!
//! These tests are currently **disabled by default** in Cargo.toml to avoid CI infrastructure requirements.
//!
//! Integration tests for reinhardt-flatpages working with reinhardt-sitemaps.
//! These tests verify that flatpages integrate properly with sitemap generation
//! and other framework features.
//!
//! Based on Django's test_sitemaps.py, test_templatetags.py
//!
//! This file contains integration tests that verify reinhardt-flatpages works
//! correctly with other reinhardt crates:
//! - reinhardt-sitemaps: Sitemap generation
//! - reinhardt-templates: Template tag helpers
//! - Multi-site support
//!
//! HTTP-level integration tests (CSRF, authentication) are located in
//! tests/integration/ directory.

mod flatpages_common;

use flatpages_common::*;

#[cfg(test)]
mod flatpage_sitemap_tests {
    use super::*;
    use reinhardt_flatpages::sitemaps::get_flatpages_for_sitemap;
    use reinhardt_sitemaps::{Sitemap, SitemapItem};

    // Django test: test_flatpage_sitemap (from test_sitemaps.py)
    #[tokio::test]
    async fn test_flatpage_sitemap() {
        // Test that flatpage sitemap includes public pages
        // and excludes registration-required pages
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example Site").await;

        // Create public flatpage at /foo/
        create_test_flatpage(&pool, "/foo/", "Foo Page", "Public content", false, site_id).await;

        // Create registration-required flatpage at /private-foo/
        create_test_flatpage(
            &pool,
            "/private-foo/",
            "Private Foo",
            "Private content",
            true, // registration_required
            site_id,
        )
        .await;

        // Get flatpages for sitemap (should exclude registration-required)
        let pages = get_flatpages_for_sitemap(&pool, site_id)
            .await
            .expect("Failed to get flatpages for sitemap");

        // Assert we only got the public page
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/foo/");
        assert!(!pages[0].registration_required);

        // Generate sitemap
        let mut sitemap = Sitemap::new();
        for page in pages {
            let url = format!("http://example.com{}", page.url);
            sitemap
                .add_item(SitemapItem::new(url))
                .expect("Failed to add item");
        }

        let xml = sitemap.to_xml().expect("Failed to generate XML");

        // Assert sitemap contains /foo/ but not /private-foo/
        assert!(xml.contains("http://example.com/foo/"));
        assert!(!xml.contains("http://example.com/private-foo/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }
}

#[cfg(test)]
mod flatpage_template_tag_tests {
    use super::*;
    use reinhardt_flatpages::templatetags::{
        get_all_flatpages, get_flatpages, get_flatpages_with_prefix,
    };

    // Django tests from test_templatetags.py
    // These tests verify the underlying helper functions that would be used by template tags

    #[tokio::test]
    async fn test_get_flatpages_tag() {
        // Django test: test_get_flatpages_tag
        // Template: {% get_flatpages as flatpages %}
        // Should retrieve all non-registration-required flatpages
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        // Create public flatpages
        create_test_flatpage(
            &pool,
            "/flatpage/",
            "A Flatpage",
            "Content 1",
            false,
            site_id,
        )
        .await;
        create_test_flatpage(
            &pool,
            "/location/flatpage/",
            "A Nested Flatpage",
            "Content 2",
            false,
            site_id,
        )
        .await;

        // Create registration-required flatpages (should be excluded)
        create_test_flatpage(
            &pool,
            "/sekrit/",
            "Sekrit Flatpage",
            "Content 3",
            true,
            site_id,
        )
        .await;

        let pages = get_flatpages(&pool, site_id)
            .await
            .expect("Failed to get flatpages");

        // Should only get the 2 public flatpages
        assert_eq!(pages.len(), 2);
        assert!(pages.iter().any(|p| p.url == "/flatpage/"));
        assert!(pages.iter().any(|p| p.url == "/location/flatpage/"));
        assert!(!pages.iter().any(|p| p.url == "/sekrit/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_get_flatpages_tag_for_anon_user() {
        // Django test: test_get_flatpages_tag_for_anon_user
        // Anonymous users should only see non-registration-required flatpages
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        create_test_flatpage(&pool, "/flatpage/", "A Flatpage", "Content", false, site_id).await;
        create_test_flatpage(&pool, "/sekrit/", "Sekrit", "Content", true, site_id).await;

        // For anonymous user, use get_flatpages (excludes registration-required)
        let pages = get_flatpages(&pool, site_id)
            .await
            .expect("Failed to get flatpages");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/flatpage/");

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_get_flatpages_tag_for_authenticated_user() {
        // Django test: test_get_flatpages_tag_for_user
        // Authenticated users should see ALL flatpages
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        create_test_flatpage(&pool, "/flatpage/", "A Flatpage", "Content", false, site_id).await;
        create_test_flatpage(
            &pool,
            "/location/flatpage/",
            "Nested",
            "Content",
            false,
            site_id,
        )
        .await;
        create_test_flatpage(&pool, "/sekrit/", "Sekrit", "Content", true, site_id).await;
        create_test_flatpage(
            &pool,
            "/location/sekrit/",
            "Nested Sekrit",
            "Content",
            true,
            site_id,
        )
        .await;

        // For authenticated user, use get_all_flatpages
        let pages = get_all_flatpages(&pool, site_id)
            .await
            .expect("Failed to get all flatpages");

        // Should get all 4 flatpages
        assert_eq!(pages.len(), 4);
        assert!(pages.iter().any(|p| p.url == "/flatpage/"));
        assert!(pages.iter().any(|p| p.url == "/location/flatpage/"));
        assert!(pages.iter().any(|p| p.url == "/sekrit/"));
        assert!(pages.iter().any(|p| p.url == "/location/sekrit/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_get_flatpages_with_prefix() {
        // Django test: test_get_flatpages_with_prefix
        // Template: {% get_flatpages '/location/' as location_flatpages %}
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        create_test_flatpage(&pool, "/flatpage/", "A Flatpage", "Content", false, site_id).await;
        create_test_flatpage(
            &pool,
            "/location/flatpage/",
            "Location Flatpage",
            "Content",
            false,
            site_id,
        )
        .await;

        let pages = get_flatpages_with_prefix(&pool, site_id, "/location/", false)
            .await
            .expect("Failed to get flatpages with prefix");

        // Should only get flatpage with /location/ prefix
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/location/flatpage/");

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_get_flatpages_with_prefix_for_anon_user() {
        // Django test: test_get_flatpages_with_prefix_for_anon_user
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        create_test_flatpage(
            &pool,
            "/location/flatpage/",
            "Location",
            "Content",
            false,
            site_id,
        )
        .await;
        create_test_flatpage(
            &pool,
            "/location/sekrit/",
            "Sekrit",
            "Content",
            true,
            site_id,
        )
        .await;

        // Anonymous user - exclude registration-required
        let pages = get_flatpages_with_prefix(&pool, site_id, "/location/", false)
            .await
            .expect("Failed to get flatpages");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/location/flatpage/");

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_get_flatpages_with_prefix_for_authenticated_user() {
        // Django test: test_get_flatpages_with_prefix_for_user
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site_id = create_test_site(&pool, "example.com", "Example").await;

        create_test_flatpage(
            &pool,
            "/location/flatpage/",
            "Location",
            "Content",
            false,
            site_id,
        )
        .await;
        create_test_flatpage(
            &pool,
            "/location/sekrit/",
            "Sekrit",
            "Content",
            true,
            site_id,
        )
        .await;

        // Authenticated user - include registration-required
        let pages = get_flatpages_with_prefix(&pool, site_id, "/location/", true)
            .await
            .expect("Failed to get flatpages");

        assert_eq!(pages.len(), 2);
        assert!(pages.iter().any(|p| p.url == "/location/flatpage/"));
        assert!(pages.iter().any(|p| p.url == "/location/sekrit/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    // NOTE: test_get_flatpages_with_variable_prefix and test_get_flatpages_parsing_errors
    // are template syntax tests that require a full template engine implementation.
    // The underlying functionality (get_flatpages_with_prefix) is already tested above.
}

// NOTE: CSRF and authentication integration tests have been moved to
// tests/integration/ directory for HTTP-level testing with Axum framework

#[cfg(test)]
mod flatpage_multi_site_tests {
    use super::*;
    use reinhardt_flatpages::{FlatPage, FlatPageSite};

    #[tokio::test]
    async fn test_find_flatpages_by_site() {
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site1_id = create_test_site(&pool, "site1.com", "Site 1").await;
        let site2_id = create_test_site(&pool, "site2.com", "Site 2").await;

        // Create flatpages for site 1
        create_test_flatpage(
            &pool,
            "/site1-page/",
            "Site 1 Page",
            "Content",
            false,
            site1_id,
        )
        .await;

        // Create flatpages for site 2
        create_test_flatpage(
            &pool,
            "/site2-page/",
            "Site 2 Page",
            "Content",
            false,
            site2_id,
        )
        .await;

        // Query site 1 flatpages
        let site1_pages = FlatPageSite::find_by_site(&pool, site1_id)
            .await
            .expect("Failed to find site 1 pages");

        assert_eq!(site1_pages.len(), 1);
        assert_eq!(site1_pages[0].url, "/site1-page/");

        // Query site 2 flatpages
        let site2_pages = FlatPageSite::find_by_site(&pool, site2_id)
            .await
            .expect("Failed to find site 2 pages");

        assert_eq!(site2_pages.len(), 1);
        assert_eq!(site2_pages[0].url, "/site2-page/");

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_associate_flatpage_with_multiple_sites() {
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site1_id = create_test_site(&pool, "site1.com", "Site 1").await;
        let site2_id = create_test_site(&pool, "site2.com", "Site 2").await;

        // Create a flatpage
        let mut flatpage = FlatPage::new(
            "/shared-page/".to_string(),
            "Shared Page".to_string(),
            "Content".to_string(),
        );
        flatpage.save(&pool).await.expect("Failed to save");

        // Associate with both sites
        FlatPageSite::associate(&pool, flatpage.id, site1_id)
            .await
            .expect("Failed to associate with site1");
        FlatPageSite::associate(&pool, flatpage.id, site2_id)
            .await
            .expect("Failed to associate with site2");

        // Verify both sites have the page
        let site1_pages = FlatPageSite::find_by_site(&pool, site1_id)
            .await
            .expect("Failed to find site 1 pages");
        assert!(site1_pages.iter().any(|p| p.url == "/shared-page/"));

        let site2_pages = FlatPageSite::find_by_site(&pool, site2_id)
            .await
            .expect("Failed to find site 2 pages");
        assert!(site2_pages.iter().any(|p| p.url == "/shared-page/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }

    #[tokio::test]
    async fn test_disassociate_flatpage_from_site() {
        let pool = setup_test_db().await;
        create_test_tables(&pool).await;

        let site1_id = create_test_site(&pool, "site1.com", "Site 1").await;
        let site2_id = create_test_site(&pool, "site2.com", "Site 2").await;

        // Create flatpage associated with both sites
        let flatpage =
            create_test_flatpage(&pool, "/page/", "Page", "Content", false, site1_id).await;

        FlatPageSite::associate(&pool, flatpage.id, site2_id)
            .await
            .expect("Failed to associate with site2");

        // Disassociate from site1
        FlatPageSite::disassociate(&pool, flatpage.id, site1_id)
            .await
            .expect("Failed to disassociate");

        // Verify site1 no longer has the page
        let site1_pages = FlatPageSite::find_by_site(&pool, site1_id)
            .await
            .expect("Failed to find site 1 pages");
        assert!(!site1_pages.iter().any(|p| p.url == "/page/"));

        // Verify site2 still has the page
        let site2_pages = FlatPageSite::find_by_site(&pool, site2_id)
            .await
            .expect("Failed to find site 2 pages");
        assert!(site2_pages.iter().any(|p| p.url == "/page/"));

        // Cleanup
        clear_flatpages(&pool).await;
        cleanup_test_tables(&pool).await;
    }
}
