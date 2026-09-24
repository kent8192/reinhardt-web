//! Routable Discovery, JWKS, authorization, token, and UserInfo endpoints.

use super::protocol::{OidcAuthorizationRequest, OidcError, OidcPendingHandle, OidcProvider};
use crate::oauth2_server::OAuthBrowserSession;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Request, Response};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use url::form_urlencoded;

/// Host-owned login, reauthentication, consent, and account-selection flow.
///
/// The host calls OidcProvider::complete_authorization with the same browser
/// session binding after interaction. For prompt=none it must avoid UI and
/// complete with an appropriate OIDC error when silent approval is impossible.
#[async_trait]
pub trait OidcInteraction: Send + Sync {
	/// Present or silently complete a validated pending OIDC request.
	async fn present(
		&self,
		request: Request,
		pending: OidcPendingHandle,
	) -> reinhardt_core::exception::Result<Response>;
}

/// Endpoint to mount at the matching configured URL.
#[derive(Clone, Copy, Debug)]
pub enum OidcEndpoint {
	/// Browser Authorization Code request.
	Authorization,
	/// Code redemption and ID Token issuance.
	Token,
	/// Opaque-token UserInfo response.
	UserInfo,
	/// OpenID Provider Discovery metadata.
	Discovery,
	/// Public signing keys.
	Jwks,
}

/// Reinhardt HTTP handler for one OpenID Provider endpoint.
pub struct OidcHandler {
	/// Endpoint implemented by this handler.
	pub endpoint: OidcEndpoint,
	provider: Arc<OidcProvider>,
	interaction: Option<Arc<dyn OidcInteraction>>,
}

impl OidcHandler {
	/// Create a token, UserInfo, Discovery, or JWKS handler.
	pub fn new(provider: Arc<OidcProvider>, endpoint: OidcEndpoint) -> Self {
		Self {
			endpoint,
			provider,
			interaction: None,
		}
	}

	/// Create an authorization handler with host-owned interaction.
	pub fn authorization(
		provider: Arc<OidcProvider>,
		interaction: Arc<dyn OidcInteraction>,
	) -> Self {
		Self {
			endpoint: OidcEndpoint::Authorization,
			provider,
			interaction: Some(interaction),
		}
	}

	async fn authorization_request(
		&self,
		request: Request,
	) -> reinhardt_core::exception::Result<Response> {
		if request.method != Method::GET {
			return Ok(method_not_allowed("GET"));
		}
		let Some(interaction) = &self.interaction else {
			return Ok(oidc_error(OidcError::ServerError));
		};
		let Some(binding) = request.extensions.get::<OAuthBrowserSession>() else {
			return Ok(oidc_error(OidcError::ServerError));
		};
		let params = match parse_params(request.uri.query().unwrap_or_default().as_bytes()) {
			Ok(params) => params,
			Err(error) => return Ok(oidc_error(error)),
		};
		let client_id = params.get("client_id").cloned().unwrap_or_default();
		let redirect_uri = params.get("redirect_uri").cloned().unwrap_or_default();
		let state = params.get("state").cloned();
		if ["request", "request_uri", "claims"]
			.iter()
			.any(|key| params.contains_key(*key))
			|| params
				.get("response_mode")
				.is_some_and(|mode| mode != "query")
		{
			return Ok(self
				.authorization_error(
					&client_id,
					&redirect_uri,
					state.as_deref(),
					OidcError::InvalidRequest,
				)
				.await);
		}
		let max_age = match params.get("max_age") {
			Some(value) => match value.parse::<u64>() {
				Ok(value) if value <= i64::MAX as u64 => Some(value),
				_ => {
					return Ok(self
						.authorization_error(
							&client_id,
							&redirect_uri,
							state.as_deref(),
							OidcError::InvalidRequest,
						)
						.await);
				}
			},
			None => None,
		};
		let input = OidcAuthorizationRequest {
			client_id: client_id.clone(),
			redirect_uri: redirect_uri.clone(),
			response_type: params.get("response_type").cloned().unwrap_or_default(),
			scope: params.get("scope").cloned().unwrap_or_default(),
			code_challenge: params.get("code_challenge").cloned().unwrap_or_default(),
			code_challenge_method: params
				.get("code_challenge_method")
				.cloned()
				.unwrap_or_default(),
			state: state.clone(),
			nonce: params.get("nonce").cloned(),
			prompt: params.get("prompt").cloned(),
			max_age,
		};
		let browser_session = binding.0.clone();
		match self
			.provider
			.begin_authorization(input, &browser_session)
			.await
		{
			Ok(pending) => interaction.present(request, pending).await.map(no_store),
			Err(error) => Ok(self
				.authorization_error(&client_id, &redirect_uri, state.as_deref(), error)
				.await),
		}
	}

	async fn authorization_error(
		&self,
		client_id: &str,
		redirect_uri: &str,
		state: Option<&str>,
		error: OidcError,
	) -> Response {
		match self
			.provider
			.authorization_error_redirect(client_id, redirect_uri, state, error)
			.await
		{
			Some(location) => redirect(&location),
			None => oidc_error(error),
		}
	}

	async fn token_request(&self, request: Request) -> Response {
		if request.method != Method::POST {
			return method_not_allowed("POST");
		}
		let params = match form_params(&request) {
			Ok(params) => params,
			Err(error) => return oidc_error(error),
		};
		if params.contains_key("client_id") || params.contains_key("client_secret") {
			return oidc_error(OidcError::InvalidClient);
		}
		let Some((client_id, secret)) = basic_auth(&request) else {
			return oidc_error(OidcError::InvalidClient);
		};
		match params.get("grant_type").map(String::as_str) {
			Some("authorization_code") => {}
			Some("") | None => return oidc_error(OidcError::InvalidRequest),
			Some(_) => return oidc_error(OidcError::UnsupportedGrantType),
		}
		let required = |key| {
			params
				.get(key)
				.map(String::as_str)
				.filter(|v| !v.is_empty())
		};
		let (Some(code), Some(redirect_uri), Some(verifier)) = (
			required("code"),
			required("redirect_uri"),
			required("code_verifier"),
		) else {
			return oidc_error(OidcError::InvalidRequest);
		};
		match self
			.provider
			.exchange_code(code, &client_id, &secret, redirect_uri, verifier)
			.await
		{
			Ok(response) => json_response(StatusCode::OK, json!(response)),
			Err(error) => oidc_error(error),
		}
	}

	async fn userinfo_request(&self, request: Request) -> Response {
		if request.method != Method::GET && request.method != Method::POST {
			return method_not_allowed("GET, POST");
		}
		let token = request
			.headers
			.get("authorization")
			.and_then(|header| header.to_str().ok())
			.and_then(|value| auth_credentials(value, "Bearer"));
		let Some(token) = token.filter(|value| !value.is_empty()) else {
			return invalid_userinfo_token();
		};
		match self.provider.userinfo(token).await {
			Ok(Some(info)) => json_response(StatusCode::OK, info),
			Ok(None) => invalid_userinfo_token(),
			Err(_) => oidc_error(OidcError::ServerError),
		}
	}

	async fn jwks_request(&self, request: Request) -> Response {
		if request.method != Method::GET {
			return method_not_allowed("GET");
		}
		match self.provider.jwks().await {
			Ok(value) => json_response(StatusCode::OK, value),
			Err(error) => oidc_error(error),
		}
	}
}

#[async_trait]
impl Handler for OidcHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		if self.provider.is_production() && !request.is_secure() {
			return Ok(no_store(Response::new(StatusCode::BAD_REQUEST)));
		}
		let client_ip = request.get_client_ip();
		if self.provider.is_production() && client_ip.is_none() {
			return Ok(no_store(Response::new(StatusCode::BAD_REQUEST)));
		}
		let key = format!(
			"oidc:{:?}:{}",
			self.endpoint,
			client_ip.map_or_else(|| "unknown".to_owned(), |ip| ip.to_string()),
		);
		if !self.provider.allow(&key).await {
			return Ok(no_store(Response::new(StatusCode::TOO_MANY_REQUESTS)));
		}
		match self.endpoint {
			OidcEndpoint::Authorization => self.authorization_request(request).await,
			OidcEndpoint::Token => Ok(self.token_request(request).await),
			OidcEndpoint::UserInfo => Ok(self.userinfo_request(request).await),
			OidcEndpoint::Discovery => Ok(if request.method == Method::GET {
				json_response(StatusCode::OK, self.provider.discovery())
			} else {
				method_not_allowed("GET")
			}),
			OidcEndpoint::Jwks => Ok(self.jwks_request(request).await),
		}
	}
}

fn parse_params(bytes: &[u8]) -> Result<HashMap<String, String>, OidcError> {
	let input = std::str::from_utf8(bytes).map_err(|_| OidcError::InvalidRequest)?;
	let mut params = HashMap::new();
	for (key, value) in form_urlencoded::parse(input.as_bytes()) {
		if params
			.insert(key.into_owned(), value.into_owned())
			.is_some()
		{
			return Err(OidcError::InvalidRequest);
		}
	}
	Ok(params)
}

fn form_params(request: &Request) -> Result<HashMap<String, String>, OidcError> {
	if !request
		.headers
		.get("content-type")
		.and_then(|header| header.to_str().ok())
		.is_some_and(|value| {
			value.split(';').next().is_some_and(|mime| {
				mime.trim()
					.eq_ignore_ascii_case("application/x-www-form-urlencoded")
			})
		}) || request.body().len() > 16_384
	{
		return Err(OidcError::InvalidRequest);
	}
	parse_params(request.body())
}

fn basic_auth(request: &Request) -> Option<(String, String)> {
	let value = request.headers.get("authorization")?.to_str().ok()?;
	let value = auth_credentials(value, "Basic")?;
	let raw = STANDARD.decode(value).ok()?;
	let raw = std::str::from_utf8(&raw).ok()?;
	let (id, secret) = raw.split_once(':')?;
	let decode = |value: &str| {
		form_urlencoded::parse(format!("v={value}").as_bytes())
			.next()
			.map(|(_, value)| value.into_owned())
	};
	Some((decode(id)?, decode(secret)?))
}

fn auth_credentials<'a>(value: &'a str, scheme: &str) -> Option<&'a str> {
	let (actual_scheme, credentials) = value.split_once(' ')?;
	(actual_scheme.eq_ignore_ascii_case(scheme) && !credentials.is_empty()).then_some(credentials)
}

fn no_store(response: Response) -> Response {
	response
		.with_header("Cache-Control", "no-store")
		.with_header("Pragma", "no-cache")
}

fn json_response(status: StatusCode, value: Value) -> Response {
	no_store(
		Response::new(status)
			.with_json(&value)
			.expect("JSON value serialization is infallible"),
	)
}

fn oidc_error(error: OidcError) -> Response {
	let status = match error {
		OidcError::InvalidClient => StatusCode::UNAUTHORIZED,
		OidcError::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
		_ => StatusCode::BAD_REQUEST,
	};
	let response = json_response(status, json!({"error":error.as_str()}));
	if error == OidcError::InvalidClient {
		response.with_header("WWW-Authenticate", "Basic realm=\"oidc\"")
	} else {
		response
	}
}

fn invalid_userinfo_token() -> Response {
	json_response(StatusCode::UNAUTHORIZED, json!({"error":"invalid_token"}))
		.with_header("WWW-Authenticate", "Bearer error=\"invalid_token\"")
}

fn redirect(location: &str) -> Response {
	no_store(Response::new(StatusCode::FOUND).with_header("Location", location))
}

fn method_not_allowed(allow: &'static str) -> Response {
	no_store(Response::new(StatusCode::METHOD_NOT_ALLOWED).with_header("Allow", allow))
}
