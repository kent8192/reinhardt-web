use crate::actions::Action;
use crate::metadata::{get_actions_for_viewset, ActionMetadata};
use crate::middleware::ViewSetMiddleware;
use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Request, Response, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// ViewSet trait - similar to Django REST Framework's ViewSet
/// Uses composition of mixins instead of inheritance
#[async_trait]
pub trait ViewSet: Send + Sync {
    /// Get the basename for URL routing
    fn get_basename(&self) -> &str;

    /// Dispatch request to appropriate action
    async fn dispatch(&self, request: Request, action: Action) -> Result<Response>;

    /// Get extra actions defined on this ViewSet
    /// Returns custom actions decorated with #[action] or manually registered
    fn get_extra_actions(&self) -> Vec<ActionMetadata> {
        let viewset_type = std::any::type_name::<Self>();

        // Try inventory-based registration first
        let mut actions = get_actions_for_viewset(viewset_type);

        // Also check manual registration
        let manual_actions = crate::registry::get_registered_actions(viewset_type);
        actions.extend(manual_actions);

        actions
    }

    /// Get URL map for extra actions
    /// Returns empty map for uninitialized ViewSets
    fn get_extra_action_url_map(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Get current base URL (only available after initialization)
    fn get_current_base_url(&self) -> Option<String> {
        None
    }

    /// Reverse an action name to a URL
    fn reverse_action(&self, _action_name: &str, _args: &[&str]) -> Result<String> {
        Err(reinhardt_apps::Error::NotFound(
            "ViewSet not bound to router".to_string(),
        ))
    }

    /// Get middleware for this ViewSet
    /// Returns None if no middleware is configured
    fn get_middleware(&self) -> Option<Arc<dyn ViewSetMiddleware>> {
        None
    }

    /// Check if login is required for this ViewSet
    fn requires_login(&self) -> bool {
        false
    }

    /// Get required permissions for this ViewSet
    fn get_required_permissions(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Generic ViewSet implementation
/// Composes functionality through trait bounds
#[allow(dead_code)]
pub struct GenericViewSet<T> {
    basename: String,
    handler: T,
}

impl<T: 'static> GenericViewSet<T> {
    /// Creates a new `GenericViewSet` with the given basename and handler.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::GenericViewSet;
    ///
    /// let viewset = GenericViewSet::new("users", ());
    /// assert_eq!(viewset.get_basename(), "users");
    /// ```
    pub fn new(basename: impl Into<String>, handler: T) -> Self {
        Self {
            basename: basename.into(),
            handler,
        }
    }

    /// Convert ViewSet to Handler with action mapping
    /// Returns a ViewSetBuilder for configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_viewsets::{GenericViewSet, viewset_actions};
    /// use hyper::Method;
    ///
    /// let viewset = GenericViewSet::new("users", ());
    /// let actions = viewset_actions!(GET => "list");
    /// let handler = viewset.as_view().with_actions(actions).build();
    /// ```
    pub fn as_view(self) -> crate::builder::ViewSetBuilder<Self>
    where
        T: Send + Sync,
    {
        crate::builder::ViewSetBuilder::new(self)
    }
}

#[async_trait]
impl<T: Send + Sync> ViewSet for GenericViewSet<T> {
    fn get_basename(&self) -> &str {
        &self.basename
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        // Default implementation delegates to mixins if available
        // This would be extended with actual mixin dispatch logic
        Err(reinhardt_apps::Error::NotFound(
            "Action not implemented".to_string(),
        ))
    }
}

/// ModelViewSet - combines all CRUD mixins
/// Similar to Django REST Framework's ModelViewSet but using composition
pub struct ModelViewSet<M, S> {
    basename: String,
    _model: std::marker::PhantomData<M>,
    _serializer: std::marker::PhantomData<S>,
}

impl<M: 'static, S: 'static> ModelViewSet<M, S> {
    /// Creates a new `ModelViewSet` with the given basename.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::ModelViewSet;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: Option<i64>,
    ///     username: String,
    /// }
    ///
    /// impl Model for User {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "users" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let viewset = ModelViewSet::<User, reinhardt_serializers::JsonSerializer<User>>::new("users");
    /// assert_eq!(viewset.get_basename(), "users");
    /// ```
    pub fn new(basename: impl Into<String>) -> Self {
        Self {
            basename: basename.into(),
            _model: std::marker::PhantomData,
            _serializer: std::marker::PhantomData,
        }
    }

    /// Convert ViewSet to Handler with action mapping
    /// Returns a ViewSetBuilder for configuration
    pub fn as_view(self) -> crate::builder::ViewSetBuilder<Self>
    where
        M: Send + Sync,
        S: Send + Sync,
    {
        crate::builder::ViewSetBuilder::new(self)
    }
}

#[async_trait]
impl<M, S> ViewSet for ModelViewSet<M, S>
where
    M: Send + Sync,
    S: Send + Sync,
{
    fn get_basename(&self) -> &str {
        &self.basename
    }

    async fn dispatch(&self, request: Request, action: Action) -> Result<Response> {
        // Route to appropriate handler based on HTTP method and action
        match (request.method.clone(), action.detail) {
            (Method::GET, false) => {
                // List action
                self.handle_list(request).await
            }
            (Method::GET, true) => {
                // Retrieve action
                self.handle_retrieve(request).await
            }
            (Method::POST, false) => {
                // Create action
                self.handle_create(request).await
            }
            (Method::PUT, true) | (Method::PATCH, true) => {
                // Update action
                self.handle_update(request).await
            }
            (Method::DELETE, true) => {
                // Destroy action
                self.handle_destroy(request).await
            }
            _ => Err(reinhardt_apps::Error::Http(
                "Method not allowed".to_string(),
            )),
        }
    }
}

impl<M, S> ModelViewSet<M, S>
where
    M: Send + Sync,
    S: Send + Sync,
{
    async fn handle_list(&self, _request: Request) -> Result<Response> {
        // Implementation would query all objects and serialize them
        Response::ok()
            .with_json(&serde_json::json!([]))
            .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
    }

    async fn handle_retrieve(&self, _request: Request) -> Result<Response> {
        // Implementation would get object by ID and serialize it
        Response::ok()
            .with_json(&serde_json::json!({}))
            .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
    }

    async fn handle_create(&self, _request: Request) -> Result<Response> {
        // Implementation would deserialize, validate, and create object
        Response::created()
            .with_json(&serde_json::json!({}))
            .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
    }

    async fn handle_update(&self, _request: Request) -> Result<Response> {
        // Implementation would deserialize, validate, and update object
        Response::ok()
            .with_json(&serde_json::json!({}))
            .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
    }

    async fn handle_destroy(&self, _request: Request) -> Result<Response> {
        // Implementation would delete object
        Ok(Response::no_content())
    }
}

/// ReadOnlyModelViewSet - only list and retrieve
/// Demonstrates selective composition of mixins
pub struct ReadOnlyModelViewSet<M, S> {
    basename: String,
    _model: std::marker::PhantomData<M>,
    _serializer: std::marker::PhantomData<S>,
}

impl<M: 'static, S: 'static> ReadOnlyModelViewSet<M, S> {
    /// Creates a new `ReadOnlyModelViewSet` with the given basename.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::ReadOnlyModelViewSet;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: Option<i64>,
    ///     username: String,
    /// }
    ///
    /// impl Model for User {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "users" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let viewset = ReadOnlyModelViewSet::<User, reinhardt_serializers::JsonSerializer<User>>::new("users");
    /// assert_eq!(viewset.get_basename(), "users");
    /// ```
    pub fn new(basename: impl Into<String>) -> Self {
        Self {
            basename: basename.into(),
            _model: std::marker::PhantomData,
            _serializer: std::marker::PhantomData,
        }
    }

    /// Convert ViewSet to Handler with action mapping
    /// Returns a ViewSetBuilder for configuration
    pub fn as_view(self) -> crate::builder::ViewSetBuilder<Self>
    where
        M: Send + Sync,
        S: Send + Sync,
    {
        crate::builder::ViewSetBuilder::new(self)
    }
}

#[async_trait]
impl<M, S> ViewSet for ReadOnlyModelViewSet<M, S>
where
    M: Send + Sync,
    S: Send + Sync,
{
    fn get_basename(&self) -> &str {
        &self.basename
    }

    async fn dispatch(&self, request: Request, action: Action) -> Result<Response> {
        match (request.method.clone(), action.detail) {
            (Method::GET, false) => {
                // List only
                Response::ok()
                    .with_json(&serde_json::json!([]))
                    .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
            }
            (Method::GET, true) => {
                // Retrieve only
                Response::ok()
                    .with_json(&serde_json::json!({}))
                    .map_err(|e| reinhardt_apps::Error::Http(e.to_string()))
            }
            _ => Err(reinhardt_apps::Error::Http(
                "Method not allowed".to_string(),
            )),
        }
    }
}
