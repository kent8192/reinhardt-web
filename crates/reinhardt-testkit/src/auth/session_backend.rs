//! Native session/account adapter for trusted test fixture writes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use reinhardt_auth::{AuthBackend, AuthIdentity, AuthenticationError};
use reinhardt_http::Request;
use reinhardt_middleware::session::{AsyncSessionBackend, SessionData};

use super::SessionIdentity;

/// Session storage and current-account lookup for cookie-authentication tests.
///
/// Wrap a session store and pass the same adapter to `SessionAuthBuilder` and
/// both backends of `CookieSessionAuthMiddleware`. Successful trusted fixture
/// saves publish the user's identity to this adapter's separate account registry.
/// The latest successful fixture save defines that user's current privileges for
/// all their sessions. Direct underlying-store edits do not update the registry;
/// failed saves do not publish an identity. Destroying a session does not delete
/// the account or affect the user's other sessions.
///
/// This is a scoped test fixture. Production authentication must resolve accounts
/// from its authoritative user store. Only trusted fixture code should save
/// through this adapter. See [`super::SessionAuthBuilder`] for executable wiring.
///
/// # Platform parity
///
/// **P0 (native only)**: this type and its constructor are absent on
/// `wasm32-unknown-unknown`, matching the native session builders and backends.
/// Approved RC addition: <https://github.com/kent8192/reinhardt-web/issues/6765>.
pub struct TestSessionAuthBackend {
	sessions: Arc<dyn AsyncSessionBackend>,
	accounts: Mutex<HashMap<String, SessionIdentity>>,
}

impl TestSessionAuthBackend {
	/// Wrap a session store with a fresh, scoped current-account registry.
	///
	/// **P0 (native only)**, with the same target boundary as the adapter type.
	pub fn new(sessions: Arc<dyn AsyncSessionBackend>) -> Self {
		Self {
			sessions,
			accounts: Mutex::new(HashMap::new()),
		}
	}
}

#[async_trait]
impl AsyncSessionBackend for TestSessionAuthBackend {
	async fn load(&self, id: &str) -> reinhardt_http::Result<Option<SessionData>> {
		self.sessions.load(id).await
	}

	async fn save(&self, session: &SessionData) -> reinhardt_http::Result<()> {
		self.sessions.save(session).await?;
		if let Some(id) = session.data.get("user_id").and_then(|value| value.as_str()) {
			let identity = SessionIdentity {
				user_id: id.to_owned(),
				is_staff: session
					.data
					.get("is_staff")
					.and_then(|value| value.as_bool())
					.unwrap_or(false),
				is_superuser: session
					.data
					.get("is_superuser")
					.and_then(|value| value.as_bool())
					.unwrap_or(false),
			};
			self.accounts
				.lock()
				.unwrap_or_else(std::sync::PoisonError::into_inner)
				.insert(id.to_owned(), identity);
		}
		Ok(())
	}

	async fn destroy(&self, id: &str) -> reinhardt_http::Result<()> {
		self.sessions.destroy(id).await
	}

	async fn touch(&self, id: &str, ttl: Duration) -> reinhardt_http::Result<()> {
		self.sessions.touch(id, ttl).await
	}
}

struct FixtureAccount(SessionIdentity);

impl AuthIdentity for FixtureAccount {
	fn id(&self) -> String {
		self.0.user_id.clone()
	}
	fn is_authenticated(&self) -> bool {
		true
	}
	fn is_admin(&self) -> bool {
		self.0.is_staff || self.0.is_superuser
	}
}

#[async_trait]
impl AuthBackend for TestSessionAuthBackend {
	async fn authenticate(
		&self,
		_request: &Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		// Cookie middleware loads the session and calls get_user with its user ID.
		Ok(None)
	}

	async fn get_user(
		&self,
		id: &str,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		Ok(self
			.accounts
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.get(id)
			.cloned()
			.map(|identity| Box::new(FixtureAccount(identity)) as Box<dyn AuthIdentity>))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde_json::json;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::time::SystemTime;

	#[derive(Default)]
	struct SessionStore {
		data: Mutex<HashMap<String, SessionData>>,
		fail: AtomicBool,
	}
	#[async_trait]
	impl AsyncSessionBackend for SessionStore {
		async fn load(&self, id: &str) -> reinhardt_http::Result<Option<SessionData>> {
			Ok(self.data.lock().unwrap().get(id).cloned())
		}
		async fn save(&self, session: &SessionData) -> reinhardt_http::Result<()> {
			if self.fail.load(Ordering::Acquire) {
				return Err(reinhardt_http::Error::Internal("store unavailable".into()));
			}
			self.data
				.lock()
				.unwrap()
				.insert(session.id.clone(), session.clone());
			Ok(())
		}
		async fn destroy(&self, id: &str) -> reinhardt_http::Result<()> {
			self.data.lock().unwrap().remove(id);
			Ok(())
		}
		async fn touch(&self, id: &str, ttl: Duration) -> reinhardt_http::Result<()> {
			self.data.lock().unwrap().get_mut(id).unwrap().expires_at = SystemTime::now() + ttl;
			Ok(())
		}
	}

	#[rstest]
	#[tokio::test]
	async fn session_operations_delegate_without_removing_current_accounts() {
		// Arrange: two sessions for one scoped account.
		let store = Arc::new(SessionStore::default());
		let backend = TestSessionAuthBackend::new(store.clone());
		let identity = SessionIdentity {
			user_id: "shared".into(),
			is_staff: true,
			is_superuser: false,
		};
		backend
			.save(&identity.to_session_data("one", Duration::from_secs(1)))
			.await
			.unwrap();
		backend
			.save(&identity.to_session_data("two", Duration::from_secs(1)))
			.await
			.unwrap();
		let before = backend.load("two").await.unwrap().unwrap().expires_at;

		// Act: refresh one session and destroy the other through the adapter.
		backend.touch("two", Duration::from_secs(60)).await.unwrap();
		backend.destroy("one").await.unwrap();

		// Assert: real storage operations run; other sessions and the account survive.
		assert!(store.load("one").await.unwrap().is_none());
		assert!(backend.load("two").await.unwrap().unwrap().expires_at > before);
		assert!(
			backend
				.get_user("shared")
				.await
				.unwrap()
				.unwrap()
				.is_admin()
		);
		assert!(
			TestSessionAuthBackend::new(store)
				.get_user("shared")
				.await
				.unwrap()
				.is_none()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn failed_replacement_save_preserves_the_last_successful_account() {
		// Arrange: a successful regular profile followed by an unavailable store.
		let store = Arc::new(SessionStore::default());
		let backend = TestSessionAuthBackend::new(store.clone());
		let mut session = SessionIdentity {
			user_id: "regular".into(),
			is_staff: false,
			is_superuser: false,
		}
		.to_session_data("session", Duration::from_secs(60));
		backend.save(&session).await.unwrap();
		store.fail.store(true, Ordering::Release);
		session.data.insert("is_staff".into(), json!(true));

		// Act / Assert: a failed privilege change cannot update the live account.
		assert!(backend.save(&session).await.is_err());
		assert!(
			!backend
				.get_user("regular")
				.await
				.unwrap()
				.unwrap()
				.is_admin()
		);
		assert_eq!(
			backend.load("session").await.unwrap().unwrap().data["is_staff"],
			json!(false)
		);
	}
}
