// Integration tests for redirect middleware
// These tests require reinhardt-redirects + reinhardt-middleware integration

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_contrib::{Site, SiteManager};
use reinhardt_exception::Error;
use reinhardt_http::{Request, Response};
use reinhardt_middleware::{RedirectFallbackMiddleware, RedirectResponseConfig};
use reinhardt_redirects::{Redirect, RedirectManager};
use reinhardt_types::Handler;
use std::sync::Arc;

struct TestHandler {
    status: StatusCode,
}

impl TestHandler {
    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
        }
    }

    fn ok() -> Self {
        Self {
            status: StatusCode::OK,
        }
    }
}

#[async_trait]
impl Handler for TestHandler {
    async fn handle(&self, _request: Request) -> reinhardt_apps::Result<Response> {
        Ok(Response::new(self.status))
    }
}

fn setup_test_environment() -> (RedirectManager, SiteManager) {
    let redirect_manager = RedirectManager::new();
    let site_manager = SiteManager::new();

    // Add default site
    site_manager.add_site(Site::new(1, "example.com", "Example Site"));

    (redirect_manager, site_manager)
}

#[cfg(test)]
mod redirect_middleware_tests {
    use super::*;

    /// Tests that RedirectFallbackMiddleware raises ImproperlyConfigured
    /// when sites framework is not installed (Django: test_sites_not_installed)
    #[test]
    fn test_sites_not_installed() {
        let redirect_manager = Arc::new(RedirectManager::new());

        // Attempt to create middleware without sites framework (None)
        let result = RedirectFallbackMiddleware::new(redirect_manager, None, 1);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::ImproperlyConfigured(_)));

        // Check error message
        let msg = err.to_string();
        assert!(msg.contains("Sites framework"));
    }

    /// Tests custom response class for "gone" responses (Django: test_response_gone_class)
    /// Returns 403 instead of default 410
    #[tokio::test]
    async fn test_custom_response_gone_class() {
        let (mut redirect_manager, site_manager) = setup_test_environment();

        // Add a "gone" redirect (empty new_path)
        let redirect = Redirect::new("/initial/", "").for_site(1);
        redirect_manager.add(redirect);

        // Create custom response config that returns 403 for gone responses
        let config = RedirectResponseConfig {
            gone_fn: Response::forbidden,
            redirect_fn: |location, status| match status {
                301 => Response::permanent_redirect(location),
                302 => Response::temporary_redirect(location),
                307 => Response::temporary_redirect_preserve_method(location),
                _ => Response::permanent_redirect(location),
            },
        };

        let middleware = RedirectFallbackMiddleware::new(
            Arc::new(redirect_manager),
            Some(Arc::new(site_manager)),
            1,
        )
        .unwrap()
        .with_response_config(config);

        use reinhardt_types::Middleware;
        let handler = Arc::new(TestHandler::not_found()) as Arc<dyn Handler>;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/initial/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Verify response is 403 (Forbidden) instead of 410 (Gone)
        assert_eq!(response.status, StatusCode::FORBIDDEN);
    }

    /// Tests redirects work with APPEND_SLASH setting (Django: test_redirect_with_append_slash)
    #[tokio::test]
    async fn test_redirect_with_append_slash() {
        let (mut redirect_manager, site_manager) = setup_test_environment();

        // Add redirect with trailing slash
        let redirect = Redirect::new("/initial/", "/new_target/").for_site(1);
        redirect_manager.add(redirect);

        let middleware = RedirectFallbackMiddleware::new(
            Arc::new(redirect_manager),
            Some(Arc::new(site_manager)),
            1,
        )
        .unwrap()
        .with_append_slash(true); // Enable APPEND_SLASH

        use reinhardt_types::Middleware;
        let handler = Arc::new(TestHandler::not_found()) as Arc<dyn Handler>;
        // Request WITHOUT trailing slash
        let request = Request::new(
            Method::GET,
            Uri::from_static("/initial"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Should redirect to "/new_target/" with status 301
        assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
        assert_eq!(response.headers.get("location").unwrap(), "/new_target/");
    }

    /// Tests query string handling (Django: test_redirect_with_append_slash_and_query_string)
    #[tokio::test]
    async fn test_redirect_with_query_string() {
        let (mut redirect_manager, site_manager) = setup_test_environment();

        // Add redirect that includes query string in old_path
        let redirect = Redirect::new("/initial/?foo", "/new_target/").for_site(1);
        redirect_manager.add(redirect);

        let middleware = RedirectFallbackMiddleware::new(
            Arc::new(redirect_manager),
            Some(Arc::new(site_manager)),
            1,
        )
        .unwrap()
        .with_append_slash(true);

        use reinhardt_types::Middleware;
        let handler = Arc::new(TestHandler::not_found()) as Arc<dyn Handler>;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/initial?foo"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Should redirect to "/new_target/"
        assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
        assert_eq!(response.headers.get("location").unwrap(), "/new_target/");
    }

    /// Tests middleware short-circuits on non-404 responses
    /// (Django: test_redirect_shortcircuits_non_404_response)
    #[tokio::test]
    async fn test_middleware_shortcircuits_on_success() {
        let (redirect_manager, site_manager) = setup_test_environment();

        let middleware = RedirectFallbackMiddleware::new(
            Arc::new(redirect_manager),
            Some(Arc::new(site_manager)),
            1,
        )
        .unwrap();

        use reinhardt_types::Middleware;
        // Handler returns 200 OK
        let handler = Arc::new(TestHandler::ok()) as Arc<dyn Handler>;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Middleware should not interfere with successful responses
        assert_eq!(response.status, StatusCode::OK);
    }

    // NOTE: FastAPI Swagger UI tests are integration tests specific to API documentation
    // and are not applicable to the core redirect functionality. They test:
    // - test_swagger_ui: OAuth2 redirect URL configuration in Swagger UI
    // - test_swagger_ui_oauth2_redirect: OAuth2 redirect endpoint rendering
    // These would require reinhardt-browsable-api or similar documentation framework.
}
