//! Main dispatcher for handling HTTP requests

use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};

use crate::handler::BaseHandler;

/// Main dispatcher for handling HTTP requests
pub struct Dispatcher {
    handler: BaseHandler,
}

impl Dispatcher {
    /// Create a new dispatcher with a base handler
    pub fn new(handler: BaseHandler) -> Self {
        Self { handler }
    }

    /// Dispatch a request to the appropriate handler
    pub async fn dispatch(&self, request: Request) -> Result<Response> {
        self.handler
            .handle_request(request)
            .await
            .map_err(|e| reinhardt_exception::Error::Internal(format!("Dispatch error: {}", e)))
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new(BaseHandler::default())
    }
}
