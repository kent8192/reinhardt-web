use async_trait::async_trait;

use crate::AuthenticationError;
use crate::core::AuthIdentity;

/// Unified authentication backend trait
///
/// Implement this trait to create custom authentication backends.
/// A backend handles user authentication (login) and user retrieval.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{AuthBackend, AuthIdentity, AuthenticationError};
/// use async_trait::async_trait;
/// use reinhardt_http::Request;
///
/// struct MyAuthBackend;
///
/// #[async_trait]
/// impl AuthBackend for MyAuthBackend {
///     async fn authenticate(
///         &self,
///         request: &Request,
///     ) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
///         // Extract credentials from the request and verify
///         Ok(None)
///     }
///
///     async fn get_user(
///         &self,
///         user_id: &str,
///     ) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
///         Ok(None)
///     }
/// }
/// ```
#[async_trait]
pub trait AuthBackend: Send + Sync {
	/// Authenticate a request and return a user identity if successful
	///
	/// # Arguments
	///
	/// * `request` - The incoming HTTP request
	///
	/// # Returns
	///
	/// - `Ok(Some(identity))` if authentication succeeded
	/// - `Ok(None)` if authentication failed but should try next backend
	/// - `Err(error)` if a fatal error occurred
	async fn authenticate(
		&self,
		request: &reinhardt_http::Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError>;

	/// Get a user by their ID
	///
	/// # Arguments
	///
	/// * `user_id` - The user's unique identifier
	///
	/// # Returns
	///
	/// - `Ok(Some(identity))` if user was found
	/// - `Ok(None)` if user doesn't exist
	/// - `Err(error)` if an error occurred
	async fn get_user(
		&self,
		user_id: &str,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError>;
}

/// Composite auth backend - tries multiple backends in order
///
/// This backend allows you to configure multiple authentication backends
/// and try them in sequence until one succeeds.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::CompositeAuthBackend;
///
/// let backend = CompositeAuthBackend::new();
/// // Add custom backends with backend.add_backend(Box::new(my_backend))
/// ```
pub struct CompositeAuthBackend {
	backends: Vec<Box<dyn AuthBackend>>,
}

impl CompositeAuthBackend {
	/// Creates a new composite authentication backend with no backends
	pub fn new() -> Self {
		Self {
			backends: Vec::new(),
		}
	}

	/// Adds an authentication backend to the composite
	///
	/// Backends are tried in the order they are added.
	pub fn add_backend(&mut self, backend: Box<dyn AuthBackend>) {
		self.backends.push(backend);
	}
}

impl Default for CompositeAuthBackend {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl AuthBackend for CompositeAuthBackend {
	async fn authenticate(
		&self,
		request: &reinhardt_http::Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		for backend in &self.backends {
			if let Some(user) = backend.authenticate(request).await? {
				return Ok(Some(user));
			}
		}
		Ok(None)
	}

	async fn get_user(
		&self,
		user_id: &str,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		for backend in &self.backends {
			if let Some(user) = backend.get_user(user_id).await? {
				return Ok(Some(user));
			}
		}
		Ok(None)
	}
}
