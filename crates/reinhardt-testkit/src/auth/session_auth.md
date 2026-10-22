
# Complete cookie-authentication fixture

`TestSessionAuthBackend` wraps a session store and records current account
privileges after successful trusted fixture saves. Wire the same adapter into the
session builder and both middleware backends. Direct underlying-store metadata
changes do not update the separate account registry. The latest successful fixture
save for a user defines that user's current privileges for all their sessions.
This native adapter is for tests; production backends resolve accounts from their
authoritative user store.

```rust
use async_trait::async_trait;
use reinhardt_http::{AuthState, Handler, MiddlewareChain, Request, Response};
use reinhardt_middleware::CookieSessionAuthMiddleware;
use reinhardt_middleware::session::{AsyncSessionBackend, SessionData};
use reinhardt_testkit::APIClient;
use reinhardt_testkit::auth::{ForceLoginUser, TestSessionAuthBackend};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct SessionStore(Mutex<HashMap<String, SessionData>>);
#[async_trait]
impl AsyncSessionBackend for SessionStore {
    async fn load(&self, id: &str) -> reinhardt_http::Result<Option<SessionData>> {
        Ok(self.0.lock().unwrap().get(id)
            .filter(|session| session.expires_at > SystemTime::now()).cloned())
    }
    async fn save(&self, session: &SessionData) -> reinhardt_http::Result<()> {
        self.0.lock().unwrap().insert(session.id.clone(), session.clone());
        Ok(())
    }
    async fn destroy(&self, id: &str) -> reinhardt_http::Result<()> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
    async fn touch(&self, id: &str, ttl: Duration) -> reinhardt_http::Result<()> {
        if let Some(session) = self.0.lock().unwrap().get_mut(id) {
            session.expires_at = SystemTime::now() + ttl;
        }
        Ok(())
    }
}

struct RegularUser;
impl ForceLoginUser for RegularUser {
    fn session_user_id(&self) -> String { "example-user".into() }
}

struct Endpoint;
#[async_trait]
impl Handler for Endpoint {
    async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
        let state = request.extensions.get::<AuthState>().unwrap();
        Response::ok().with_json(&json!({"admin": state.is_admin()}))
    }
}

# tokio::runtime::Runtime::new().unwrap().block_on(async {
let session_store = Arc::new(SessionStore::default());
let backend = Arc::new(TestSessionAuthBackend::new(session_store));
let middleware = CookieSessionAuthMiddleware::new(backend.clone())
    .with_auth_backend(backend.clone());
let handler = MiddlewareChain::new(Arc::new(Endpoint))
    .with_middleware(Arc::new(middleware));
let client = APIClient::from_handler(handler);

client.auth().session(&RegularUser, backend)
    .with_staff(true).apply().await.unwrap();
let response = client.get("/admin-check").await.unwrap();
assert_eq!(response.json_value().unwrap(), json!({"admin": true}));
# });
```
