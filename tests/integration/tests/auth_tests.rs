//! Authentication integration tests for flatpages
//! Based on Django's flatpages_tests authentication-related tests

use reinhardt_flatpages::FlatPage;
use reinhardt_integration_tests::{
    cleanup_test_tables, create_flatpages_tables,
    flatpages_app::{build_flatpages_app_with_auth, build_flatpages_app_without_auth},
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
    registration_required: bool,
    site_id: i64,
) {
    let mut page = FlatPage::new(url.to_string(), title.to_string(), content.to_string());
    page.registration_required = registration_required;
    page.save(pool).await.expect("Failed to save flatpage");

    sqlx::query("INSERT INTO flatpage_sites (flatpage_id, site_id) VALUES ($1, $2)")
        .bind(page.id)
        .bind(site_id)
        .execute(pool)
        .await
        .expect("Failed to associate flatpage with site");
}

#[tokio::test]
async fn test_public_flatpage_no_auth_required() {
    // Public flatpages should be accessible without authentication
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/public/",
        "Public Page",
        "Anyone can see this",
        false, // registration_required = false
        site_id,
    )
    .await;

    let app = build_flatpages_app_with_auth(pool.clone());
    let (status, body) = make_request(app, "GET", "/flatpage_root/public/", None).await;

    assert_eq!(status, 200);
    assert!(body.contains("Anyone can see this"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_registration_required_redirects_to_login() {
    // Django test: test_view_authenticated_flatpage (unauthenticated part)
    // Accessing registration-required page should redirect to login when not authenticated
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/sekrit/",
        "Sekrit Flatpage",
        "Isn't it sekrit!",
        true, // registration_required = true
        site_id,
    )
    .await;

    // Use app without authentication
    let app = build_flatpages_app_without_auth(pool.clone());
    let (status, _) = make_request(app, "GET", "/flatpage_root/sekrit/", None).await;

    // Should redirect (302 Found)
    // In a real app this would redirect to /accounts/login/?next=/flatpage_root/sekrit/
    assert_eq!(status, 302);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_authenticated_user_can_access_registration_required() {
    // Django test: test_view_authenticated_flatpage (authenticated part)
    // Authenticated users should be able to access registration-required pages
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/sekrit/",
        "Sekrit Flatpage",
        "Isn't it sekrit!",
        true, // registration_required = true
        site_id,
    )
    .await;

    // Use app with authentication
    let app = build_flatpages_app_with_auth(pool.clone());
    let (status, body) = make_request(app, "GET", "/flatpage_root/sekrit/", None).await;

    assert_eq!(status, 200);
    assert!(body.contains("Isn't it sekrit!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_fallback_authenticated_flatpage() {
    // Django test: test_fallback_authenticated_flatpage
    // Fallback middleware should also handle registration-required pages
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/sekrit/",
        "Sekrit Flatpage",
        "Isn't it sekrit!",
        true,
        site_id,
    )
    .await;

    // Unauthenticated request should redirect
    let app_without_auth = build_flatpages_app_without_auth(pool.clone());
    let (status, _) = make_request(app_without_auth, "GET", "/sekrit/", None).await;
    assert_eq!(status, 302); // Redirect to login

    // Authenticated request should succeed
    let app_with_auth = build_flatpages_app_with_auth(pool.clone());
    let (status, body) = make_request(app_with_auth, "GET", "/sekrit/", None).await;
    assert_eq!(status, 200);
    assert!(body.contains("Isn't it sekrit!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_nested_registration_required_flatpage() {
    // Test registration-required nested flatpages
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/location/sekrit/",
        "Sekrit Nested Flatpage",
        "Isn't it sekrit and deep!",
        true,
        site_id,
    )
    .await;

    // Unauthenticated user should be redirected
    let app = build_flatpages_app_without_auth(pool.clone());
    let (status, _) = make_request(app, "GET", "/flatpage_root/location/sekrit/", None).await;
    assert_eq!(status, 302);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_anonymous_user_vs_authenticated_user() {
    // Compare behavior for anonymous vs authenticated users
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;

    // Create public page
    create_flatpage(
        &pool,
        "/public/",
        "Public",
        "Public content",
        false,
        site_id,
    )
    .await;

    // Create private page
    create_flatpage(
        &pool,
        "/private/",
        "Private",
        "Private content",
        true,
        site_id,
    )
    .await;

    // Anonymous user: can access public, cannot access private
    let app_without_auth = build_flatpages_app_without_auth(pool.clone());
    let (status, _) = make_request(app_without_auth.clone(), "GET", "/public/", None).await;
    assert_eq!(status, 200);

    let (status, _) = make_request(app_without_auth, "GET", "/private/", None).await;
    assert_eq!(status, 302); // Redirect to login

    // Authenticated user: can access both
    let app_with_auth = build_flatpages_app_with_auth(pool.clone());
    let (status, _) = make_request(app_with_auth.clone(), "GET", "/public/", None).await;
    assert_eq!(status, 200);

    let (status, _) = make_request(app_with_auth, "GET", "/private/", None).await;
    assert_eq!(status, 200);

    cleanup_test_tables(&pool).await;
}

// NOTE: Full authentication integration requires:
// 1. reinhardt-auth middleware adapted for tower/axum
// 2. User model and authentication backend
// 3. Session management
// 4. Login/logout handlers
// 5. Permission checking
//
// These tests demonstrate the expected behavior and serve as
// integration targets for when auth middleware is HTTP-framework ready.
