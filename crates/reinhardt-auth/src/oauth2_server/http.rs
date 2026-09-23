//! Routable HTTP endpoints for the OAuth server.

use super::protocol::{
	AuthorizationRequest, OAuthError, OAuthServer, PendingAuthorization, TokenPrincipal,
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Request, Response};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use url::form_urlencoded;

/// Browser-session binding inserted into the request by host middleware.
#[derive(Clone, Debug)]
pub struct OAuthBrowserSession(pub String);

/// Host-owned login and consent presentation for a validated pending request.
#[async_trait]
pub trait OAuthConsentPresenter: Send + Sync {
	/// Present a pending request without approving it automatically.
	async fn present(
		&self,
		request: Request,
		pending: PendingAuthorization,
	) -> reinhardt_core::exception::Result<Response>;
}

/// An OAuth endpoint to mount at the URL advertised in metadata.
#[derive(Clone, Copy, Debug)]
pub enum OAuthEndpoint {
	/// Browser authorization request.
	Authorization,
	/// Access-token issuance.
	Token,
	/// Client-owned token revocation.
	Revocation,
	/// Resource-server token introspection.
	Introspection,
	/// Authorization-server metadata.
	Metadata,
}

/// Reinhardt HTTP handler for one OAuth protocol endpoint.
pub struct OAuthHandler {
	/// Endpoint implemented by this handler.
	pub endpoint: OAuthEndpoint,
	server: Arc<OAuthServer>,
	presenter: Option<Arc<dyn OAuthConsentPresenter>>,
}
impl OAuthHandler {
	/// Create a handler for token, revocation, introspection, or metadata.
	pub fn new(server: Arc<OAuthServer>, endpoint: OAuthEndpoint) -> Self {
		Self {
			server,
			endpoint,
			presenter: None,
		}
	}
	/// Create an authorization handler with a host-owned login/consent presenter.
	pub fn authorization(
		server: Arc<OAuthServer>,
		presenter: Arc<dyn OAuthConsentPresenter>,
	) -> Self {
		Self {
			server,
			endpoint: OAuthEndpoint::Authorization,
			presenter: Some(presenter),
		}
	}
	async fn authorization_request(
		&self,
		request: Request,
	) -> reinhardt_core::exception::Result<Response> {
		if request.method != Method::GET {
			return Ok(method_not_allowed());
		}
		let Some(browser_session) = request.extensions.get::<OAuthBrowserSession>() else {
			return Ok(oauth_error(
				OAuthError::InvalidRequest,
				StatusCode::BAD_REQUEST,
			));
		};
		let Some(presenter) = &self.presenter else {
			return Ok(oauth_error(
				OAuthError::ServerError,
				StatusCode::INTERNAL_SERVER_ERROR,
			));
		};
		let params = match parse_params(request.uri.query().unwrap_or_default().as_bytes()) {
			Ok(p) => p,
			Err(e) => return Ok(oauth_error(e, StatusCode::BAD_REQUEST)),
		};
		let get = |name: &str| params.get(name).cloned().unwrap_or_default();
		let input = AuthorizationRequest {
			client_id: get("client_id"),
			redirect_uri: get("redirect_uri"),
			response_type: get("response_type"),
			code_challenge: get("code_challenge"),
			code_challenge_method: get("code_challenge_method"),
			scope: params.get("scope").cloned(),
			resource: params.get("resource").cloned(),
			state: params.get("state").cloned(),
		};
		let binding = browser_session.0.clone();
		match self
			.server
			.begin_authorization(input.clone(), &binding)
			.await
		{
			Ok(pending) => presenter.present(request, pending).await,
			Err(error) => {
				if let Some(location) = self
					.server
					.authorization_error_redirect(
						&input.client_id,
						&input.redirect_uri,
						input.state.as_deref(),
						error,
					)
					.await
				{
					Ok(redirect(&location))
				} else {
					Ok(oauth_error(error, StatusCode::BAD_REQUEST))
				}
			}
		}
	}
	async fn token_request(&self, request: Request) -> Response {
		if request.method != Method::POST {
			return method_not_allowed();
		}
		let params = match form_params(&request) {
			Ok(p) => p,
			Err(e) => return oauth_error(e, StatusCode::BAD_REQUEST),
		};
		let credentials = match client_auth(&request, &params) {
			Ok(v) => v,
			Err(e) => return oauth_error(e, StatusCode::UNAUTHORIZED),
		};
		let (client_id, secret) = credentials;
		let response = match params.get("grant_type").map(String::as_str) {
			Some("authorization_code") => {
				if !["code", "redirect_uri", "code_verifier"]
					.iter()
					.all(|key| params.get(*key).is_some_and(|v| !v.is_empty()))
				{
					return oauth_error(OAuthError::InvalidRequest, StatusCode::BAD_REQUEST);
				}
				self.server
					.exchange_code(
						params.get("code").map(String::as_str).unwrap_or_default(),
						&client_id,
						secret.as_deref(),
						params
							.get("redirect_uri")
							.map(String::as_str)
							.unwrap_or_default(),
						params
							.get("code_verifier")
							.map(String::as_str)
							.unwrap_or_default(),
						params.get("resource").map(String::as_str),
					)
					.await
			}
			Some("client_credentials") => match secret.as_deref() {
				Some(secret) => {
					self.server
						.client_credentials(
							&client_id,
							secret,
							params.get("scope").map(String::as_str),
							params.get("resource").map(String::as_str),
						)
						.await
				}
				None => Err(OAuthError::InvalidClient),
			},
			None => Err(OAuthError::InvalidRequest),
			Some(_) => Err(OAuthError::UnsupportedGrantType),
		};
		let response = match response {
			Ok(token) => json_response(StatusCode::OK, json!(token)),
			Err(error) => oauth_error(
				error,
				if error == OAuthError::InvalidClient {
					StatusCode::UNAUTHORIZED
				} else {
					StatusCode::BAD_REQUEST
				},
			),
		};
		self.cors(response, &request, &client_id).await
	}
	async fn revocation_request(&self, request: Request) -> Response {
		if request.method != Method::POST {
			return method_not_allowed();
		}
		let params = match form_params(&request) {
			Ok(p) => p,
			Err(e) => return oauth_error(e, StatusCode::BAD_REQUEST),
		};
		let (client_id, secret) = match client_auth(&request, &params) {
			Ok(v) => v,
			Err(e) => return oauth_error(e, StatusCode::UNAUTHORIZED),
		};
		let token = params.get("token").map(String::as_str).unwrap_or_default();
		if token.is_empty() {
			return oauth_error(OAuthError::InvalidRequest, StatusCode::BAD_REQUEST);
		}
		let response = match self
			.server
			.revoke(token, &client_id, secret.as_deref())
			.await
		{
			Ok(()) => no_store(Response::new(StatusCode::OK)),
			Err(error) => oauth_error(
				error,
				if error == OAuthError::InvalidClient {
					StatusCode::UNAUTHORIZED
				} else {
					StatusCode::BAD_REQUEST
				},
			),
		};
		self.cors(response, &request, &client_id).await
	}
	async fn introspection_request(&self, request: Request) -> Response {
		if request.method != Method::POST {
			return method_not_allowed();
		}
		let params = match form_params(&request) {
			Ok(p) => p,
			Err(e) => return oauth_error(e, StatusCode::BAD_REQUEST),
		};
		if params.contains_key("client_id") || params.contains_key("client_secret") {
			return oauth_error(OAuthError::InvalidClient, StatusCode::UNAUTHORIZED);
		}
		let (resource_id, secret) = match basic_auth(&request) {
			Some(v) => v,
			None => return oauth_error(OAuthError::InvalidClient, StatusCode::UNAUTHORIZED),
		};
		let token = params.get("token").map(String::as_str).unwrap_or_default();
		if token.is_empty() {
			return oauth_error(OAuthError::InvalidRequest, StatusCode::BAD_REQUEST);
		}
		match self.server.introspect(token, &resource_id, &secret).await {
			Ok(Some(info)) => {
				let (sub, kind) = match info.principal {
					TokenPrincipal::User(id) => (id, "user"),
					TokenPrincipal::Client(id) => (id, "client"),
				};
				json_response(
					StatusCode::OK,
					json!({"active":true,"client_id":info.client_id,"sub":sub,"token_principal":kind,"scope":info.scopes.join(" "),"aud":info.audience,"iss":self.server.config().issuer,"iat":info.issued_at,"exp":info.expires_at,"token_type":"Bearer"}),
				)
			}
			Ok(None) => json_response(StatusCode::OK, json!({"active":false})),
			Err(error) => oauth_error(
				error,
				if error == OAuthError::InvalidClient {
					StatusCode::UNAUTHORIZED
				} else {
					StatusCode::INTERNAL_SERVER_ERROR
				},
			),
		}
	}
	async fn metadata_request(&self, request: Request) -> Response {
		if request.method != Method::GET {
			return method_not_allowed();
		}
		let c = self.server.config();
		let response = json_response(
			StatusCode::OK,
			json!({"issuer":c.issuer,"authorization_endpoint":c.authorization_endpoint,"token_endpoint":c.token_endpoint,"revocation_endpoint":c.revocation_endpoint,"introspection_endpoint":c.introspection_endpoint,"response_types_supported":["code"],"grant_types_supported":["authorization_code","client_credentials"],"token_endpoint_auth_methods_supported":["client_secret_basic","none"],"revocation_endpoint_auth_methods_supported":["client_secret_basic","none"],"introspection_endpoint_auth_methods_supported":["client_secret_basic"],"code_challenge_methods_supported":["S256"],"authorization_response_iss_parameter_supported":true}),
		);
		self.registered_origin(response, &request).await
	}
	async fn registered_origin(&self, response: Response, request: &Request) -> Response {
		let Some(origin) = request.headers.get("origin").and_then(|h| h.to_str().ok()) else {
			return response;
		};
		if self.server.allows_origin(origin).await.unwrap_or(false) {
			response
				.with_header("Access-Control-Allow-Origin", origin)
				.with_header("Vary", "Origin")
		} else {
			response
		}
	}
	async fn preflight(&self, request: &Request) -> Response {
		let response = self
			.registered_origin(no_store(Response::new(StatusCode::NO_CONTENT)), request)
			.await;
		if response.headers.contains_key("Access-Control-Allow-Origin") {
			response
				.with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
				.with_header(
					"Access-Control-Allow-Headers",
					"Content-Type, Authorization",
				)
				.with_header("Access-Control-Max-Age", "600")
		} else {
			no_store(Response::new(StatusCode::FORBIDDEN))
		}
	}
	async fn cors(&self, response: Response, request: &Request, client_id: &str) -> Response {
		let Some(origin) = request.headers.get("origin").and_then(|h| h.to_str().ok()) else {
			return response;
		};
		let Ok(Some(client)) = self.server.client(client_id).await else {
			return response;
		};
		if client.kind == super::store::ClientKind::Public
			&& client.browser_origins.iter().any(|o| o == origin)
		{
			response
				.with_header("Access-Control-Allow-Origin", origin)
				.with_header("Vary", "Origin")
		} else {
			response
		}
	}
}
#[async_trait]
impl Handler for OAuthHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		if self.server.is_production() && !request.is_secure {
			return Ok(no_store(Response::new(StatusCode::BAD_REQUEST)));
		}
		let key = format!(
			"{:?}:{}",
			self.endpoint,
			request
				.remote_addr
				.map(|a| a.ip().to_string())
				.unwrap_or_default()
		);
		if !self.server.allow(&key).await {
			return Ok(no_store(Response::new(StatusCode::TOO_MANY_REQUESTS)));
		}
		if request.method == Method::OPTIONS
			&& matches!(
				self.endpoint,
				OAuthEndpoint::Token | OAuthEndpoint::Revocation | OAuthEndpoint::Metadata
			) {
			return Ok(self.preflight(&request).await);
		}
		Ok(match self.endpoint {
			OAuthEndpoint::Authorization => return self.authorization_request(request).await,
			OAuthEndpoint::Token => self.token_request(request).await,
			OAuthEndpoint::Revocation => self.revocation_request(request).await,
			OAuthEndpoint::Introspection => self.introspection_request(request).await,
			OAuthEndpoint::Metadata => self.metadata_request(request).await,
		})
	}
}
fn parse_params(bytes: &[u8]) -> Result<HashMap<String, String>, OAuthError> {
	let input = std::str::from_utf8(bytes).map_err(|_| OAuthError::InvalidRequest)?;
	let mut out = HashMap::new();
	for (key, value) in form_urlencoded::parse(input.as_bytes()) {
		if out.insert(key.into_owned(), value.into_owned()).is_some() {
			return Err(OAuthError::InvalidRequest);
		}
	}
	Ok(out)
}
fn form_params(request: &Request) -> Result<HashMap<String, String>, OAuthError> {
	if !request
		.headers
		.get("content-type")
		.and_then(|h| h.to_str().ok())
		.is_some_and(|v| {
			v.split(';').next().is_some_and(|mime| {
				mime.trim()
					.eq_ignore_ascii_case("application/x-www-form-urlencoded")
			})
		}) {
		return Err(OAuthError::InvalidRequest);
	}
	if request.body().len() > 16384 {
		return Err(OAuthError::InvalidRequest);
	}
	parse_params(request.body())
}
fn basic_auth(request: &Request) -> Option<(String, String)> {
	let value = request
		.headers
		.get("authorization")?
		.to_str()
		.ok()?
		.strip_prefix("Basic ")?;
	let raw = STANDARD.decode(value).ok()?;
	let raw = std::str::from_utf8(&raw).ok()?;
	let (id, secret) = raw.split_once(':')?;
	let decode = |v: &str| {
		form_urlencoded::parse(format!("v={v}").as_bytes())
			.next()
			.map(|(_, v)| v.into_owned())
	};
	Some((decode(id)?, decode(secret)?))
}
fn client_auth(
	request: &Request,
	params: &HashMap<String, String>,
) -> Result<(String, Option<String>), OAuthError> {
	if request.headers.contains_key("authorization") {
		if params.contains_key("client_id") || params.contains_key("client_secret") {
			return Err(OAuthError::InvalidRequest);
		}
		let (id, secret) = basic_auth(request).ok_or(OAuthError::InvalidClient)?;
		Ok((id, Some(secret)))
	} else {
		if params.contains_key("client_secret") {
			return Err(OAuthError::InvalidClient);
		}
		Ok((
			params
				.get("client_id")
				.cloned()
				.ok_or(OAuthError::InvalidClient)?,
			None,
		))
	}
}
fn no_store(response: Response) -> Response {
	response
		.with_header("Cache-Control", "no-store")
		.with_header("Pragma", "no-cache")
}
fn json_response(status: StatusCode, value: serde_json::Value) -> Response {
	no_store(
		Response::new(status)
			.with_json(&value)
			.expect("serializing a JSON value cannot fail"),
	)
}
fn oauth_error(error: OAuthError, status: StatusCode) -> Response {
	let response = json_response(status, json!({"error":error.as_str()}));
	if error == OAuthError::InvalidClient {
		response.with_header("WWW-Authenticate", "Basic realm=\"oauth\"")
	} else {
		response
	}
}
fn redirect(location: &str) -> Response {
	no_store(Response::new(StatusCode::FOUND).with_header("Location", location))
}
fn method_not_allowed() -> Response {
	no_store(Response::new(StatusCode::METHOD_NOT_ALLOWED))
}
