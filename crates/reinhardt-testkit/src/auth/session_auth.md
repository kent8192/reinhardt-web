
# Complete cookie-authentication fixture

This native example uses only the existing backend traits. The private fixture
records account privileges on trusted builder saves and resolves them separately
on each request. Changes made directly to stored session metadata do not update
the account registry. The latest trusted fixture save for a user defines that
user's current privileges for all their sessions. This fixture is for tests;
production backends must resolve accounts from their authoritative user store.

```rust
use async_trait::async_trait;
use reinhardt_auth::{AuthBackend, AuthIdentity, AuthenticationError};
use reinhardt_http::{AuthState, Handler, MiddlewareChain, Request, Response};
use reinhardt_middleware::CookieSessionAuthMiddleware;
use reinhardt_middleware::session::{AsyncSessionBackend, SessionData};
use reinhardt_testkit::APIClient;
use reinhardt_testkit::auth::{ForceLoginUser, SessionIdentity};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct FixtureState {
    sessions: HashMap<String, SessionData>,
    accounts: HashMap<String, SessionIdentity>,
}

#[derive(Default)]
struct FixtureBackend(Mutex<FixtureState>);

struct FixtureAccount(SessionIdentity);
impl AuthIdentity for FixtureAccount {
    fn id(&self) -> String { self.0.user_id.clone() }
    fn is_authenticated(&self) -> bool { true }
    fn is_admin(&self) -> bool { self.0.is_staff || self.0.is_superuser }
}

#[async_trait]
impl AsyncSessionBackend for FixtureBackend {
    async fn load(&self, id: &str) -> reinhardt_http::Result<Option<SessionData>> {
        Ok(self.0.lock().unwrap().sessions.get(id)
            .filter(|session| session.expires_at > SystemTime::now()).cloned())
    }
    async fn save(&self, session: &SessionData) -> reinhardt_http::Result<()> {
        let mut state = self.0.lock().unwrap();
        state.sessions.insert(session.id.clone(), session.clone());
        // Trusted fixture writes publish the current account independently.
        if let Some(id) = session.data.get("user_id").and_then(|value| value.as_str()) {
            state.accounts.insert(id.to_owned(), SessionIdentity {
                user_id: id.to_owned(),
                is_staff: session.data.get("is_staff")
                    .and_then(|value| value.as_bool()).unwrap_or(false),
                is_superuser: session.data.get("is_superuser")
                    .and_then(|value| value.as_bool()).unwrap_or(false),
            });
        }
        Ok(())
    }
    async fn destroy(&self, id: &str) -> reinhardt_http::Result<()> {
        self.0.lock().unwrap().sessions.remove(id);
        Ok(())
    }
    async fn touch(&self, id: &str, ttl: Duration) -> reinhardt_http::Result<()> {
        if let Some(session) = self.0.lock().unwrap().sessions.get_mut(id) {
            session.expires_at = SystemTime::now() + ttl;
        }
        Ok(())
    }
}

#[async_trait]
impl AuthBackend for FixtureBackend {
    async fn authenticate(&self, _request: &Request)
        -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
        Ok(None) // Cookie middleware performs the session lookup.
    }
    async fn get_user(&self, id: &str)
        -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
        Ok(self.0.lock().unwrap().accounts.get(id).cloned()
            .map(|account| Box::new(FixtureAccount(account)) as Box<dyn AuthIdentity>))
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
let backend = Arc::new(FixtureBackend::default());
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
