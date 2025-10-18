//! Test ViewSet implementations for middleware testing

use crate::middleware::{CompositeMiddleware, ViewSetMiddleware};
use crate::{Action, ViewSet};
use async_trait::async_trait;
use reinhardt_apps::{Request, Response, Result};
use std::sync::Arc;

/// Test ViewSet with configurable middleware support
#[derive(Debug, Clone)]
pub struct TestViewSet {
    basename: String,
    login_required: bool,
    required_permissions: Vec<String>,
}

impl TestViewSet {
    pub fn new(basename: impl Into<String>) -> Self {
        Self {
            basename: basename.into(),
            login_required: false,
            required_permissions: Vec::new(),
        }
    }

    pub fn with_login_required(mut self, login_required: bool) -> Self {
        self.login_required = login_required;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.required_permissions = permissions;
        self
    }

    /// Convert ViewSet to Handler with action mapping
    pub fn as_view(self) -> crate::builder::ViewSetBuilder<Self> {
        crate::builder::ViewSetBuilder::new(self)
    }
}

#[async_trait]
impl ViewSet for TestViewSet {
    fn get_basename(&self) -> &str {
        &self.basename
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        // Simple test implementation that always returns success
        let mut response = Response::new(hyper::StatusCode::OK);
        response.body = "Test response".into();
        Ok(response)
    }

    fn get_middleware(&self) -> Option<Arc<dyn ViewSetMiddleware>> {
        if self.login_required || !self.required_permissions.is_empty() {
            let mut composite = CompositeMiddleware::new();

            if self.login_required {
                composite = composite.with_authentication(true);
            }

            if !self.required_permissions.is_empty() {
                composite = composite.with_permissions(self.required_permissions.clone());
            }

            Some(Arc::new(composite))
        } else {
            None
        }
    }

    fn requires_login(&self) -> bool {
        self.login_required
    }

    fn get_required_permissions(&self) -> Vec<String> {
        self.required_permissions.clone()
    }
}

/// Simple ViewSet for testing without middleware
#[derive(Debug, Clone)]
pub struct SimpleViewSet {
    basename: String,
}

impl SimpleViewSet {
    pub fn new(basename: impl Into<String>) -> Self {
        Self {
            basename: basename.into(),
        }
    }
}

#[async_trait]
impl ViewSet for SimpleViewSet {
    fn get_basename(&self) -> &str {
        &self.basename
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        let mut response = Response::new(hyper::StatusCode::OK);
        response.body = "Simple response".into();
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{HeaderMap, Method, Uri, Version};
    use std::collections::HashMap;

    fn create_test_request() -> Request {
        Request::new(
            Method::GET,
            Uri::from_static("/test/"),
            Version::HTTP_11,
            HeaderMap::new(),
            bytes::Bytes::new(),
        )
    }

    #[tokio::test]
    async fn test_viewset_without_middleware() {
        let viewset = TestViewSet::new("test");
        assert!(!viewset.requires_login());
        assert!(viewset.get_required_permissions().is_empty());
        assert!(viewset.get_middleware().is_none());

        let request = create_test_request();
        let action = Action::list();
        let response = viewset.dispatch(request, action).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_viewset_with_login_required() {
        let viewset = TestViewSet::new("test").with_login_required(true);
        assert!(viewset.requires_login());
        assert!(viewset.get_middleware().is_some());

        let request = create_test_request();
        let action = Action::list();
        let response = viewset.dispatch(request, action).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_viewset_with_permissions() {
        let permissions = vec!["read".to_string(), "write".to_string()];
        let viewset = TestViewSet::new("test").with_permissions(permissions.clone());
        assert_eq!(viewset.get_required_permissions(), permissions);
        assert!(viewset.get_middleware().is_some());
    }
}
