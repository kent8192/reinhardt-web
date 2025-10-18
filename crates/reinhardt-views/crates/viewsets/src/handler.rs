use crate::middleware::ViewSetMiddleware;
/// ViewSetHandler - wraps a ViewSet as a Handler
use crate::{Action, ViewSet};
use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Handler, Request, Response, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Handler implementation that wraps a ViewSet
pub struct ViewSetHandler<V: ViewSet> {
    viewset: Arc<V>,
    action_map: HashMap<Method, String>,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    suffix: Option<String>,

    /// // Attributes set after as_view() is called
    /// // These mirror Django REST Framework's behavior
    args: RwLock<Option<Vec<String>>>,
    kwargs: RwLock<Option<HashMap<String, String>>>,
    has_handled_request: RwLock<bool>,
}

impl<V: ViewSet> ViewSetHandler<V> {
    pub fn new(
        viewset: Arc<V>,
        action_map: HashMap<Method, String>,
        name: Option<String>,
        suffix: Option<String>,
    ) -> Self {
        Self {
            viewset,
            action_map,
            name,
            suffix,
            args: RwLock::new(None),
            kwargs: RwLock::new(None),
            has_handled_request: RwLock::new(false),
        }
    }

    /// Check if args attribute is set (for testing)
    pub fn has_args(&self) -> bool {
        self.args.read().unwrap().is_some()
    }

    /// Check if kwargs attribute is set (for testing)
    pub fn has_kwargs(&self) -> bool {
        self.kwargs.read().unwrap().is_some()
    }

    /// Check if request attribute is set (for testing)
    pub fn has_request(&self) -> bool {
        *self.has_handled_request.read().unwrap()
    }

    /// Check if action_map is set (for testing)
    pub fn has_action_map(&self) -> bool {
        !self.action_map.is_empty()
    }
}

#[async_trait]
impl<V: ViewSet + 'static> Handler for ViewSetHandler<V> {
    async fn handle(&self, mut request: Request) -> Result<Response> {
        // Set attributes when handling request (DRF behavior)
        *self.has_handled_request.write().unwrap() = true;
        *self.args.write().unwrap() = Some(Vec::new());

        // Extract path parameters from URI
        let kwargs = extract_path_params(&request);
        *self.kwargs.write().unwrap() = Some(kwargs);

        // Process middleware before ViewSet
        if let Some(middleware) = self.viewset.get_middleware() {
            if let Some(response) = middleware.process_request(&mut request).await? {
                return Ok(response);
            }
        }

        // Resolve action from HTTP method
        let action_name = self.action_map.get(&request.method).ok_or_else(|| {
            reinhardt_apps::Error::Http(format!("Method {} not allowed", request.method))
        })?;

        // Create Action from name
        let action = Action::from_name(action_name);

        // Dispatch to ViewSet
        let response = self.viewset.dispatch(request, action).await?;

        // Process middleware after ViewSet
        // Note: We can't use the original request here since it was moved to dispatch
        // In a real implementation, we might need to clone the request or restructure this
        Ok(response)
    }
}

/// Extract path parameters from request
/// Simple implementation - in production would use router's path matching
fn extract_path_params(request: &Request) -> HashMap<String, String> {
    let mut params = HashMap::new();

    /// // Simple extraction: if path has pattern like /resource/123/
    /// // extract "123" as the "id" parameter
    let path = request.uri.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    /// // If we have at least 2 segments, assume second is an ID
    if segments.len() >= 2 {
        // Check if second segment looks like a numeric ID
        if segments[1].parse::<i64>().is_ok() || !segments[1].is_empty() {
            params.insert("id".to_string(), segments[1].to_string());
        }
    }

    params
}
