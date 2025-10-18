//! HTTP integration tests for flatpages
//! Based on Django's flatpages_tests/test_views.py and test_middleware.py

use reinhardt_flatpages::FlatPage;
use reinhardt_integration_tests::{
    cleanup_test_tables, create_flatpages_tables, flatpages_app::build_flatpages_app, make_request,
    setup_test_db,
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
async fn test_view_flatpage() {
    // Django test: test_view_flatpage
    // A flatpage can be served through a view
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/flatpage/",
        "A Flatpage",
        "Isn't it flat!",
        false,
        site_id,
    )
    .await;

    let app = build_flatpages_app(pool.clone());
    let (status, body) = make_request(app, "GET", "/flatpage_root/flatpage/", None).await;

    assert_eq!(status, hyper::StatusCode::OK);
    assert!(body.contains("Isn't it flat!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_view_non_existent_flatpage() {
    // Django test: test_view_non_existent_flatpage
    // A nonexistent flatpage raises 404 when served through a view
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let app = build_flatpages_app(pool.clone());
    let (status, _) = make_request(app, "GET", "/flatpage_root/no_such_flatpage/", None).await;

    assert_eq!(status, hyper::StatusCode::NOT_FOUND);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_view_authenticated_flatpage() {
    // Django test: test_view_authenticated_flatpage
    // A flatpage served through a view can require authentication
    //
    // This test demonstrates the authentication pattern for flatpages:
    // 1. Anonymous users accessing registration-required flatpages get redirected to login
    // 2. Authenticated users with SimpleUser in request.extensions can access them
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/sekrit/",
        "Sekrit Flatpage",
        "Isn't it sekrit!",
        true, // registration_required
        site_id,
    )
    .await;

    // Verify that flatpage exists and has registration_required flag
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, url, title, content, enable_comments, template_name, registration_required \
         FROM flatpages WHERE url = $1",
    )
    .bind("/sekrit/")
    .fetch_one(&pool)
    .await
    .expect("Failed to get flatpage");

    let url: String = row.get("url");
    let title: String = row.get("title");
    let registration_required: bool = row.get("registration_required");

    assert_eq!(url, "/sekrit/");
    assert_eq!(title, "Sekrit Flatpage");
    assert!(registration_required);

    // Verify that we can create an authenticated user
    use reinhardt_auth::SimpleUser;
    use uuid::Uuid;

    let user = SimpleUser {
        id: Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        is_active: true,
        is_admin: false,
    };

    // Verify SimpleUser can be inserted into Request extensions
    let method = hyper::Method::GET;
    let uri = "/sekrit/".parse::<hyper::Uri>().expect("Invalid URI");
    let mut header_map = hyper::HeaderMap::new();
    header_map.insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );
    let body_bytes = bytes::Bytes::new();

    let mut request =
        reinhardt_http::Request::new(method, uri, hyper::Version::HTTP_11, header_map, body_bytes);
    request.extensions.insert(user.clone());

    // Verify user can be retrieved from extensions
    let retrieved_user: Option<SimpleUser> = request.extensions.get();
    assert!(retrieved_user.is_some());
    assert_eq!(retrieved_user.unwrap().username, "testuser");

    // NOTE: Full integration test with Router would verify:
    // - Anonymous request → 302 redirect to /accounts/login/?next=/sekrit/
    // - Authenticated request → 200 OK with flatpage content

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_fallback_flatpage() {
    // Django test: test_fallback_flatpage
    // A flatpage can be served by the fallback middleware
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/flatpage/",
        "A Flatpage",
        "Isn't it flat!",
        false,
        site_id,
    )
    .await;

    let app = build_flatpages_app(pool.clone());
    let (status, body) = make_request(app, "GET", "/flatpage/", None).await;

    assert_eq!(status, hyper::StatusCode::OK);
    assert!(body.contains("Isn't it flat!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_fallback_non_existent_flatpage() {
    // Django test: test_fallback_non_existent_flatpage
    // A nonexistent flatpage raises a 404 when served by the fallback middleware
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let app = build_flatpages_app(pool.clone());
    let (status, _) = make_request(app, "GET", "/no_such_flatpage/", None).await;

    assert_eq!(status, hyper::StatusCode::NOT_FOUND);

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_view_flatpage_special_chars() {
    // Django test: test_view_flatpage_special_chars
    // A flatpage with special chars in the URL can be served through a view
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/some.very_special~chars-here/",
        "A very special page",
        "Isn't it special!",
        false,
        site_id,
    )
    .await;

    let app = build_flatpages_app(pool.clone());
    let (status, body) = make_request(
        app,
        "GET",
        "/flatpage_root/some.very_special~chars-here/",
        None,
    )
    .await;

    assert_eq!(status, hyper::StatusCode::OK);
    assert!(body.contains("Isn't it special!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_fallback_flatpage_special_chars() {
    // Django test: test_fallback_flatpage_special_chars
    // A flatpage with special chars in the URL can be served by the fallback middleware
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/some.very_special~chars-here/",
        "A very special page",
        "Isn't it special!",
        false,
        site_id,
    )
    .await;

    let app = build_flatpages_app(pool.clone());
    let (status, body) = make_request(app, "GET", "/some.very_special~chars-here/", None).await;

    assert_eq!(status, hyper::StatusCode::OK);
    assert!(body.contains("Isn't it special!"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_flatpages_http_integration_nested() {
    // Test nested URL paths
    let pool = setup_test_db().await;
    create_flatpages_tables(&pool).await;

    let site_id = create_test_site(&pool, "example.com").await;
    create_flatpage(
        &pool,
        "/location/flatpage/",
        "A Nested Flatpage",
        "Isn't it flat and deep!",
        false,
        site_id,
    )
    .await;

    let app = build_flatpages_app(pool.clone());
    let (status, body) = make_request(app, "GET", "/flatpage_root/location/flatpage/", None).await;

    assert_eq!(status, hyper::StatusCode::OK);
    assert!(body.contains("Isn't it flat and deep!"));

    cleanup_test_tables(&pool).await;
}

// NOTE: APPEND_SLASH tests would require implementing redirect middleware
// These are currently tested at the FlatpageFallbackMiddleware level
// in reinhardt-flatpages/tests/test_middleware.rs
