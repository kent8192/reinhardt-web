// Router and Authentication/Permission integration tests
// Inspired by Django REST Framework's authentication and permission system

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_auth::drf_authentication::{AuthError, AuthRequest, AuthUser, Authentication};
use reinhardt_auth::{AllowAny, IsAdminUser, IsAuthenticated, Permission, PermissionContext};
use reinhardt_exception::Result;
use reinhardt_types::{Handler, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;

// Mock handler that checks authentication
#[derive(Clone)]
struct AuthenticatedHandler {
    response_text: String,
}

impl AuthenticatedHandler {
    fn new(response_text: impl Into<String>) -> Self {
        Self {
            response_text: response_text.into(),
        }
    }
}

#[async_trait]
impl Handler for AuthenticatedHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        // Check if user is authenticated (simplified)
        let is_authenticated = request
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .is_some();

        if is_authenticated {
            Ok(Response::ok().with_body(Bytes::from(self.response_text.clone())))
        } else {
            Ok(Response::new(StatusCode::UNAUTHORIZED))
        }
    }
}

// Mock Authentication implementation
#[derive(Clone)]
struct MockAuthentication;

#[async_trait]
impl Authentication for MockAuthentication {
    async fn authenticate(
        &self,
        request: &AuthRequest,
    ) -> std::result::Result<Option<AuthUser>, AuthError> {
        // Check for Authorization header (try both cases)
        let auth_header = request
            .get_header("Authorization")
            .or_else(|| request.get_header("authorization"));

        if let Some(auth_header) = auth_header {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                if token == "valid_token" {
                    return Ok(Some(AuthUser {
                        id: 123,
                        username: "testuser".to_string(),
                        email: "test@example.com".to_string(),
                        is_active: true,
                        is_staff: false,
                    }));
                } else if token == "admin_token" {
                    return Ok(Some(AuthUser {
                        id: 1,
                        username: "admin".to_string(),
                        email: "admin@example.com".to_string(),
                        is_active: true,
                        is_staff: true,
                    }));
                } else {
                    return Err(AuthError::InvalidToken);
                }
            }
        }
        Err(AuthError::MissingCredentials)
    }
}

// Helper to convert hyper HeaderMap to HashMap
fn headers_to_map(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_string(), val.to_string()))
        })
        .collect()
}

// Test 1: Handler with authentication check
#[tokio::test]
async fn test_router_with_authentication_middleware() {
    let handler = Arc::new(AuthenticatedHandler::new("authenticated response"));

    // Test without authentication
    let request_unauth = Request::new(
        Method::GET,
        Uri::from_static("/protected/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response_unauth = handler.handle(request_unauth).await.unwrap();
    assert_eq!(response_unauth.status, StatusCode::UNAUTHORIZED);

    // Test with authentication
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer valid_token".parse().unwrap());

    let request_auth = Request::new(
        Method::GET,
        Uri::from_static("/protected/"),
        Version::HTTP_11,
        headers,
        Bytes::new(),
    );

    let response_auth = handler.handle(request_auth).await.unwrap();
    assert_eq!(response_auth.status, StatusCode::OK);
}

// Test 2: Permission checks (AllowAny, IsAuthenticated, IsAdminUser)
#[tokio::test]
async fn test_router_with_permission_classes() {
    let auth = MockAuthentication;

    // Test AllowAny permission
    let allow_any = AllowAny;
    let request = Request::new(
        Method::GET,
        Uri::from_static("/public/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let context = PermissionContext {
        request: &request,
        is_authenticated: false,
        is_admin: false,
        is_active: false,
    };

    assert!(allow_any.has_permission(&context).await);

    // Test IsAuthenticated permission - without auth
    let is_authenticated = IsAuthenticated;
    let context_unauth = PermissionContext {
        request: &request,
        is_authenticated: false,
        is_admin: false,
        is_active: false,
    };

    assert!(!is_authenticated.has_permission(&context_unauth).await);

    // Test IsAuthenticated permission - with auth
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer valid_token".parse().unwrap());

    let request_auth = Request::new(
        Method::GET,
        Uri::from_static("/users/"),
        Version::HTTP_11,
        headers.clone(),
        Bytes::new(),
    );

    // Create AuthRequest from hyper Request
    let auth_request = AuthRequest::new().with_headers(headers_to_map(&headers));

    let user_result = auth.authenticate(&auth_request).await;
    assert!(user_result.is_ok());
    assert!(user_result.unwrap().is_some());

    let context_auth = PermissionContext {
        request: &request_auth,
        is_authenticated: true,
        is_admin: false,
        is_active: true,
    };

    assert!(is_authenticated.has_permission(&context_auth).await);

    // Test IsAdminUser permission - regular user
    let is_admin = IsAdminUser;
    assert!(!is_admin.has_permission(&context_auth).await);

    // Test IsAdminUser permission - admin user
    let mut admin_headers = HeaderMap::new();
    admin_headers.insert("authorization", "Bearer admin_token".parse().unwrap());

    let request_admin = Request::new(
        Method::GET,
        Uri::from_static("/admin/"),
        Version::HTTP_11,
        admin_headers.clone(),
        Bytes::new(),
    );

    // Create AuthRequest from hyper Request
    let admin_auth_request = AuthRequest::new().with_headers(headers_to_map(&admin_headers));

    let admin_user_result = auth.authenticate(&admin_auth_request).await;
    assert!(admin_user_result.is_ok());
    let admin_user = admin_user_result.unwrap();
    assert!(admin_user.is_some());
    assert!(admin_user.unwrap().is_staff);

    let context_admin = PermissionContext {
        request: &request_admin,
        is_authenticated: true,
        is_admin: true,
        is_active: true,
    };

    assert!(is_admin.has_permission(&context_admin).await);
}

// Test 3: Combined authentication and permission checks
#[tokio::test]
async fn test_router_authentication_permission_integration() {
    let auth = MockAuthentication;

    // Simulate a route that requires authentication and admin permission
    let mut headers_user = HeaderMap::new();
    headers_user.insert("authorization", "Bearer valid_token".parse().unwrap());

    let request_user = Request::new(
        Method::DELETE,
        Uri::from_static("/users/123/"),
        Version::HTTP_11,
        headers_user.clone(),
        Bytes::new(),
    );

    // Authenticate the user
    let user_auth_request = AuthRequest::new().with_headers(headers_to_map(&headers_user));
    let user_result = auth.authenticate(&user_auth_request).await;
    assert!(user_result.is_ok());
    assert!(user_result.unwrap().is_some());

    // Check permission - regular user trying to delete
    let is_admin = IsAdminUser;
    let context_user = PermissionContext {
        request: &request_user,
        is_authenticated: true,
        is_admin: false,
        is_active: true,
    };

    assert!(!is_admin.has_permission(&context_user).await);

    // Now test with admin user
    let mut headers_admin = HeaderMap::new();
    headers_admin.insert("authorization", "Bearer admin_token".parse().unwrap());

    let request_admin = Request::new(
        Method::DELETE,
        Uri::from_static("/users/123/"),
        Version::HTTP_11,
        headers_admin.clone(),
        Bytes::new(),
    );

    // Authenticate the admin
    let admin_auth_request = AuthRequest::new().with_headers(headers_to_map(&headers_admin));
    let admin_result = auth.authenticate(&admin_auth_request).await;
    assert!(admin_result.is_ok());
    let admin_user = admin_result.unwrap();
    assert!(admin_user.is_some());
    assert!(admin_user.unwrap().is_staff);

    // Check permission - admin user can delete
    let context_admin = PermissionContext {
        request: &request_admin,
        is_authenticated: true,
        is_admin: true,
        is_active: true,
    };

    assert!(is_admin.has_permission(&context_admin).await);
}
