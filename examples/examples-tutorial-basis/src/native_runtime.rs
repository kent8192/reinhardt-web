//! Native runtime wiring for the tutorial-basis example.
//!
//! Accepted app/config/shared paths stay target-neutral. Native-only routing,
//! admin setup, middleware, DI factories, and persistence helpers live here,
//! with no-op WASM counterparts where the shared route graph needs a callable
//! function.

#[cfg(server)]
#[path = "apps/polls/admin.rs"]
pub mod polls_admin;

#[cfg(server)]
#[path = "apps/polls/urls/server_urls.rs"]
mod polls_server_urls;

#[cfg(server)]
#[path = "apps/users/urls/server_urls.rs"]
mod users_server_urls;

#[cfg(server)]
use crate::apps::users::models::User;
#[cfg(server)]
use reinhardt::Model;
#[cfg(server)]
use reinhardt::core::exception::Error;
#[cfg(server)]
use reinhardt::di::{Depends, injectable_factory};
#[cfg(server)]
use reinhardt::middleware::session::{SessionConfig, SessionData, USER_ID_SESSION_KEY};
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::{ServerRouter, UnifiedRouter};

#[cfg(server)]
pub struct PollsConfig;

#[cfg(server)]
impl reinhardt::reinhardt_apps::apps::AppLabel for PollsConfig {
	const LABEL: &'static str = "polls";
}

#[cfg(server)]
impl PollsConfig {
	pub fn config() -> reinhardt::reinhardt_apps::AppConfig {
		reinhardt::reinhardt_apps::AppConfig::new("polls", "polls")
	}
}

#[cfg(server)]
pub fn polls_server_url_patterns() -> ServerRouter {
	polls_server_urls::server_url_patterns()
}

#[cfg(not(server))]
pub fn polls_server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}

#[cfg(server)]
pub fn users_server_url_patterns() -> ServerRouter {
	users_server_urls::server_url_patterns()
}

#[cfg(not(server))]
pub fn users_server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}

#[cfg(server)]
pub fn mount_server_url_patterns(router: UnifiedRouter) -> UnifiedRouter {
	router.server(|s| {
		s.mount("/", polls_server_url_patterns())
			.mount("/", users_server_url_patterns())
	})
}

#[cfg(not(server))]
pub fn mount_server_url_patterns(router: UnifiedRouter) -> UnifiedRouter {
	router
}

#[cfg(server)]
pub fn mount_admin_routes(router: UnifiedRouter) -> UnifiedRouter {
	use crate::config::admin::configure_admin;
	use reinhardt::admin::{admin_routes_with_di, admin_static_routes};

	let admin_site = std::sync::Arc::new(configure_admin());
	let (admin_router, admin_di) = admin_routes_with_di(admin_site);
	router
		.mount("/admin/", admin_router)
		.mount("/static/admin/", admin_static_routes())
		.with_di_registrations(admin_di)
}

#[cfg(not(server))]
pub fn mount_admin_routes(router: UnifiedRouter) -> UnifiedRouter {
	router
}

#[cfg(server)]
fn create_session_middleware() -> reinhardt::middleware::session::SessionMiddleware {
	let config = SessionConfig::new(
		"sessionid".to_string(),
		std::time::Duration::from_secs(1_209_600),
	)
	.with_http_only(true)
	.with_same_site("Lax".to_string())
	.with_path("/".to_string());
	reinhardt::middleware::session::SessionMiddleware::new(config)
}

#[cfg(server)]
pub fn with_session_middleware(router: UnifiedRouter) -> UnifiedRouter {
	router.with_middleware(create_session_middleware())
}

#[cfg(not(server))]
pub fn with_session_middleware(router: UnifiedRouter) -> UnifiedRouter {
	router
}

/// Project-local `BaseUserManager<User>` implementation.
#[cfg(server)]
#[derive(Clone)]
pub struct AuthUserManager {
	db: reinhardt::DatabaseConnection,
}

#[cfg(server)]
#[injectable_factory(scope = "transient")]
async fn auth_user_manager_factory(
	#[inject] db: Depends<reinhardt::DatabaseConnection>,
) -> AuthUserManager {
	AuthUserManager { db: (*db).clone() }
}

#[cfg(server)]
impl AuthUserManager {
	async fn build_user(
		&self,
		username: &str,
		password: Option<&str>,
		extra: &std::collections::HashMap<String, serde_json::Value>,
	) -> Result<User, Error> {
		use reinhardt::BaseUser;

		let username = username.trim();
		if username.is_empty() {
			return Err(Error::Validation("Username cannot be empty".to_string()));
		}
		if username.chars().count() > 150 {
			return Err(Error::Validation(
				"Username must be 150 characters or fewer".to_string(),
			));
		}

		let manager = User::objects();
		let existing = manager
			.filter(User::field_username().eq(username.to_string()))
			.first()
			.await
			.map_err(|e| Error::Database(e.to_string()))?;
		if existing.is_some() {
			return Err(Error::Validation("Username is already taken".to_string()));
		}

		let is_active = extra
			.get("is_active")
			.and_then(|v| v.as_bool())
			.unwrap_or(true);

		let mut user = User::build()
			.username(username.to_string())
			.password_hash(None)
			.is_active(is_active)
			.is_superuser(false)
			.finish();
		if let Some(pw) = password {
			if pw.chars().count() < 8 {
				return Err(Error::Validation(
					"Password must be at least 8 characters".to_string(),
				));
			}
			user.set_password(pw)
				.map_err(|e| Error::Internal(format!("Password hashing failed: {}", e)))?;
		}
		Ok(user)
	}
}

#[cfg(server)]
#[reinhardt::core::async_trait]
impl reinhardt::reinhardt_auth::BaseUserManager<User> for AuthUserManager {
	async fn create_user(
		&mut self,
		username: &str,
		password: Option<&str>,
		extra: std::collections::HashMap<String, serde_json::Value>,
	) -> Result<User, Error> {
		let new_user = self.build_user(username, password, &extra).await?;
		User::objects()
			.create_with_conn(&self.db, &new_user)
			.await
			.map_err(|e| Error::Database(e.to_string()))
	}

	async fn create_superuser(
		&mut self,
		username: &str,
		password: Option<&str>,
		extra: std::collections::HashMap<String, serde_json::Value>,
	) -> Result<User, Error> {
		let mut new_user = self.build_user(username, password, &extra).await?;
		new_user.is_superuser = true;
		User::objects()
			.create_with_conn(&self.db, &new_user)
			.await
			.map_err(|e| Error::Database(e.to_string()))
	}
}

/// Error variants for the session-based user lookup factory.
#[derive(Clone, Debug)]
pub enum SessionError {
	Anonymous,
	Inactive,
	Unavailable(String),
}

impl From<&SessionError> for ServerFnError {
	fn from(err: &SessionError) -> Self {
		match err {
			SessionError::Anonymous => ServerFnError::server(401, "Authentication required"),
			SessionError::Inactive => ServerFnError::server(403, "User account is inactive"),
			SessionError::Unavailable(_) => {
				ServerFnError::server(500, "User lookup temporarily unavailable")
			}
		}
	}
}

#[cfg(server)]
#[injectable_factory(scope = "request")]
async fn session_user_factory(#[inject] session: SessionData) -> Result<User, SessionError> {
	let Some(user_id) = session.get::<i64>(USER_ID_SESSION_KEY) else {
		return Err(SessionError::Anonymous);
	};

	let user = match User::objects()
		.filter(User::field_id().eq(user_id))
		.first()
		.await
	{
		Ok(Some(user)) => user,
		Ok(None) => return Err(SessionError::Anonymous),
		Err(err) => {
			::tracing::warn!(
				user_id = user_id,
				error = %err,
				"session_user_factory: user lookup failed"
			);
			return Err(SessionError::Unavailable(err.to_string()));
		}
	};

	if user.is_active {
		Ok(user)
	} else {
		Err(SessionError::Inactive)
	}
}

#[cfg(server)]
pub async fn vote_internal(
	request: crate::shared::types::VoteRequest,
	db: reinhardt::DatabaseConnection,
) -> std::result::Result<crate::shared::types::ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	use reinhardt::atomic;

	let updated_choice = atomic(&db, || async {
		let choice_manager = Choice::objects();
		let mut choice = choice_manager
			.get(request.choice_id)
			.first()
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?
			.ok_or_else(|| anyhow::anyhow!("Choice not found"))?;

		if *choice.question_id() != request.question_id {
			return Err(anyhow::anyhow!("Choice does not belong to this question"));
		}

		choice.votes += 1;
		choice
			.save()
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;

		Ok(choice)
	})
	.await
	.map_err(|e| ServerFnError::application(e.to_string()))?;

	Ok(crate::shared::types::ChoiceInfo::from(updated_choice))
}

#[cfg(server)]
pub async fn require_question_author(
	question_id: i64,
	user: &User,
) -> std::result::Result<crate::apps::polls::models::Question, ServerFnError> {
	use crate::apps::polls::models::Question;

	let question = Question::objects()
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can manage its choices",
		));
	}

	Ok(question)
}
