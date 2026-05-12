//! `Injectable` implementations exposing session state to the DI layer.

use async_trait::async_trait;
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext};
use reinhardt_http::Request;
use std::sync::Arc;

use super::data::SessionData;
use super::id::{ActiveSessionId, SessionCookieName, SessionId};
use super::store::SessionStore;

/// Default session cookie name used when no `SessionCookieName` extension is present.
const DEFAULT_SESSION_COOKIE_NAME: &str = "sessionid";

/// Helper function to extract session ID from HTTP request cookies.
///
/// Searches for a cookie with the specified name in the Cookie header.
///
/// # Arguments
///
/// * `request` - The HTTP request to extract the session ID from
/// * `cookie_name` - The name of the session cookie (e.g., "sessionid")
///
/// # Returns
///
/// * `Ok(String)` - The session ID if found and valid
/// * `Err(DiError)` - If the cookie header is missing, invalid, or the session cookie is not found
fn extract_session_id_from_request(request: &Request, cookie_name: &str) -> DiResult<String> {
	let cookie_header = request
		.headers
		.get(hyper::header::COOKIE)
		.ok_or_else(|| DiError::NotFound("Cookie header not found".to_string()))?;

	let cookie_str = cookie_header
		.to_str()
		.map_err(|e| DiError::ProviderError(format!("Invalid cookie header: {}", e)))?;

	for cookie in cookie_str.split(';') {
		let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
		if parts.len() == 2 && parts[0] == cookie_name {
			return Ok(parts[1].to_string());
		}
	}

	Err(DiError::NotFound(format!(
		"Session cookie '{}' not found",
		cookie_name
	)))
}

#[async_trait]
impl Injectable for SessionData {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Get SessionStore from SingletonScope
		let store = ctx.get_singleton::<Arc<SessionStore>>().ok_or_else(|| {
			DiError::NotFound(
				"SessionStore not found in SingletonScope. \
                     Ensure SessionMiddleware is configured and its store is registered."
					.to_string(),
			)
		})?;

		// Get Request from context
		let request = ctx.get_request::<Request>().ok_or_else(|| {
			DiError::NotFound("Request not found in InjectionContext".to_string())
		})?;

		// Extract configured cookie name from request extensions.
		// Extensions::get returns an owned value, so we extract it once and
		// use a reference for the lookup to avoid additional allocation.
		let ext_cookie_name = request.extensions.get::<SessionCookieName>();
		let cookie_name = ext_cookie_name
			.as_ref()
			.map(|cn| cn.as_str())
			.unwrap_or(DEFAULT_SESSION_COOKIE_NAME);

		// Prefer the SessionId injected by SessionMiddleware (present for all requests,
		// including those without a Cookie header such as the initial login request).
		// Fall back to parsing the Cookie header for requests that bypass the middleware.
		let session_id = if let Some(sid) = request.extensions.get::<SessionId>() {
			sid.as_ref().to_string()
		} else {
			extract_session_id_from_request(&request, cookie_name)?
		};

		// Load SessionData from store, attaching the request-scoped active session
		// ID holder so `SessionData::regenerate_id` can keep the middleware's
		// `Set-Cookie` value in sync with rotations. See #3827.
		let id_holder = request.extensions.get::<ActiveSessionId>();
		let mut session = store
			.get(&session_id)
			.filter(|s| s.is_valid())
			.ok_or_else(|| {
				DiError::NotFound("Valid session not found. Session may have expired.".to_string())
			})?;
		session.id_holder = id_holder;
		Ok(session)
	}
}

/// Wrapper for `Arc<SessionStore>` to enable dependency injection
///
/// This wrapper type is necessary because we cannot implement Injectable
/// for `Arc<SessionStore>` directly due to Rust's orphan rules.
#[derive(Clone)]
pub struct SessionStoreRef(pub Arc<SessionStore>);

impl SessionStoreRef {
	/// Get a reference to the inner SessionStore
	pub fn inner(&self) -> &SessionStore {
		&self.0
	}

	/// Get a clone of the inner `Arc<SessionStore>`
	pub fn arc(&self) -> Arc<SessionStore> {
		Arc::clone(&self.0)
	}
}

#[async_trait]
impl Injectable for SessionStoreRef {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		ctx.get_singleton::<Arc<SessionStore>>()
			.map(|arc_store| SessionStoreRef(Arc::clone(&*arc_store)))
			.ok_or_else(|| {
				DiError::NotFound(
					"SessionStore not found in SingletonScope. \
                     Ensure SessionMiddleware is configured and its store is registered."
						.to_string(),
				)
			})
	}
}
