//! CSRF integration tests for flatpages
//! Based on Django's flatpages_tests/test_csrf.py

use reinhardt_flatpages::FlatPage;
use reinhardt_integration_tests::{
    cleanup_test_tables, create_flatpages_tables, flatpages_app::build_flatpages_app_with_csrf,
    make_request, setup_test_db,
};
use sqlx::Row;

async fn create_test_site(pool: &sqlx::Pool<sqlx::Postgres>, domain: &str) -> i64 {
    let row = sqlx::query("INSERT INTO sites (domain, name) VALUES ($1, $2) RETURNING id")
        .bind(domain)
        .bind(domain)
        .fetch_one(pool)
        .await
        .expect("Failed to create test site");
    row.get("id")
}

async fn create_flatpage(
    pool: &sqlx::Pool<sqlx::Postgres>,
    url: &str,
    title: &str,
    content: &str,
    site_id: i64,
) {
    let mut page = FlatPage::new(url.to_string(), title.to_string(), content.to_string());
    page.save(pool).await.expect("Failed to save flatpage");

    sqlx::query("INSERT INTO flatpage_sites (flatpage_id, site_id) VALUES ($1, $2)")
        .bind(page.id)
        .bind(site_id)
        .execute(pool)
        .await
        .expect("Failed to associate flatpage with site");
}

#[tokio::test]
async fn test_view_flatpage_get() {
    // Django test: test_view_flatpage
    // GET requests should work normally with CSRF middleware
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(&pool, "/flatpage/", "A Flatpage", "Isn't it flat!", site_id).await;

    let app_with_state = build_flatpages_app_with_csrf(pool.clone());
    let (status, body) = make_request(
        app_with_state.router,
        "GET",
        "/flatpage_root/flatpage/",
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("Isn't it flat!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_post_view_flatpage_without_csrf_token() {
    // Django test: test_post_view_flatpage
    // POSTing to a flatpage served through a view will raise a CSRF error
    // if no token is provided
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(&pool, "/flatpage/", "A Flatpage", "Isn't it flat!", site_id).await;

    let app_with_state = build_flatpages_app_with_csrf(pool.clone());
    let (status, _) = make_request(
        app_with_state.router,
        "POST",
        "/flatpage_root/flatpage/",
        None,
    )
    .await;

    // Should return 403 Forbidden due to missing CSRF token
    assert_eq!(status, 403);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_post_fallback_flatpage_without_csrf_token() {
    // Django test: test_post_fallback_flatpage
    // POSTing to a flatpage served by the middleware will raise a CSRF error
    // if no token is provided
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(&pool, "/flatpage/", "A Flatpage", "Isn't it flat!", site_id).await;

    let app_with_state = build_flatpages_app_with_csrf(pool.clone());
    let (status, _) = make_request(app_with_state.router, "POST", "/flatpage/", None).await;

    // Should return 403 Forbidden due to missing CSRF token
    assert_eq!(status, 403);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_post_unknown_page() {
    // Django test: test_post_unknown_page
    // POSTing to an unknown page isn't caught as a 403 CSRF error
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let app_with_state = build_flatpages_app_with_csrf(pool.clone());
    let (status, _) = make_request(app_with_state.router, "POST", "/no_such_page/", None).await;

    // Should return 404 Not Found, not 403 CSRF error
    assert_eq!(status, 404);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_post_with_valid_csrf_token() {
    // Test that POST with valid CSRF token succeeds
    use reinhardt_integration_tests::{
        flatpages_app::get_csrf_token_for_testing, make_request_with_headers,
    };

    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(&pool, "/flatpage/", "A Flatpage", "Isn't it flat!", site_id).await;

    // Get app with state to access CSRF token
    let app_with_state = build_flatpages_app_with_csrf(pool.clone());

    // Extract valid CSRF token from app state
    let token =
        get_csrf_token_for_testing(&app_with_state.state).expect("Failed to get CSRF token");

    // Make POST request with valid CSRF token in header
    let (status, body) = make_request_with_headers(
        app_with_state.router,
        "POST",
        "/flatpage_root/flatpage/",
        None,
        vec![("X-CSRFToken", &token)],
    )
    .await;

    // Should succeed with valid token
    assert_eq!(status, 200);
    assert!(body.contains("Isn't it flat!"));

    cleanup_test_tables(&pool).await;
}

// NOTE: Full CSRF integration requires:
// 1. reinhardt-security CSRF middleware adapted for tower/axum
// 2. Token generation and validation
// 3. Cookie/session management
// 4. Form/header token extraction
//
// These tests demonstrate the expected behavior and serve as
// integration targets for when CSRF middleware is HTTP-framework ready.
