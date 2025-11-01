//! JSON body extraction

use async_trait::async_trait;
use reinhardt_apps::Request;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug};
use std::ops::Deref;

use crate::{ParamContext, ParamError, ParamResult, extract::FromRequest};

/// Extract and deserialize JSON from request body
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct CreateUser {
///     username: String,
///     email: String,
/// }
///
/// #[endpoint(POST "/users")]
/// async fn create_user(user: Json<CreateUser>) -> Result<User> {
///     let username = &user.username;
///     let email = &user.email;
///     // ...
/// }
/// ```
pub struct Json<T>(pub T);

impl<T> Json<T> {
	/// Unwrap the Json and return the inner value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::Json;
	/// use serde::Deserialize;
	///
	/// #[derive(Deserialize, Debug, PartialEq)]
	/// struct User {
	///     username: String,
	///     age: u32,
	/// }
	///
	/// let json = Json(User {
	///     username: "alice".to_string(),
	///     age: 30,
	/// });
	/// let inner = json.into_inner();
	/// assert_eq!(inner.username, "alice");
	/// assert_eq!(inner.age, 30);
	/// ```
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> Deref for Json<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: Debug> Debug for Json<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl<T: Clone> Clone for Json<T> {
	fn clone(&self) -> Self {
		Json(self.0.clone())
	}
}

#[async_trait]
impl<T> FromRequest for Json<T>
where
	T: DeserializeOwned + Send,
{
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		// Read body bytes from request
		let body_bytes = req
			.read_body()
			.map_err(|e| ParamError::BodyError(format!("Failed to read body: {}", e)))?;

		// Deserialize JSON from body bytes
		serde_json::from_slice(&body_bytes)
			.map(Json)
			.map_err(|e| ParamError::DeserializationError(e))
	}
}
