use crate::drf_authentication::Authentication;
use crate::{AuthenticationError, SimpleUser, User};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reinhardt_core::http::Request;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
	pub sub: String, // Subject (user ID)
	pub exp: i64,    // Expiration time
	pub iat: i64,    // Issued at
	pub username: String,
}

impl Claims {
	/// Creates a new JWT Claims with user information and expiration time.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::Claims;
	/// use chrono::Duration;
	///
	/// let claims = Claims::new(
	///     "user123".to_string(),
	///     "john_doe".to_string(),
	///     Duration::hours(24)
	/// );
	///
	/// assert_eq!(claims.sub, "user123");
	/// assert_eq!(claims.username, "john_doe");
	/// assert!(claims.exp > claims.iat);
	/// ```
	pub fn new(user_id: String, username: String, expires_in: Duration) -> Self {
		let now = Utc::now();
		Self {
			sub: user_id,
			username,
			iat: now.timestamp(),
			exp: (now + expires_in).timestamp(),
		}
	}
	/// Checks if the JWT claims have expired.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::Claims;
	/// use chrono::Duration;
	///
	/// let claims = Claims::new(
	///     "user123".to_string(),
	///     "john_doe".to_string(),
	///     Duration::hours(24)
	/// );
	///
	/// assert!(!claims.is_expired());
	/// ```
	pub fn is_expired(&self) -> bool {
		Utc::now().timestamp() > self.exp
	}
}

/// JWT Authentication handler
pub struct JwtAuth {
	encoding_key: EncodingKey,
	decoding_key: DecodingKey,
	validation: Validation,
}

impl JwtAuth {
	/// Creates a new JWT authentication handler with the given secret key.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::JwtAuth;
	///
	/// let secret = b"my-secret-key-12345";
	/// let jwt_auth = JwtAuth::new(secret);
	/// ```
	pub fn new(secret: &[u8]) -> Self {
		Self {
			encoding_key: EncodingKey::from_secret(secret),
			decoding_key: DecodingKey::from_secret(secret),
			validation: Validation::default(),
		}
	}
	/// Encodes JWT claims into a token string.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::{JwtAuth, Claims};
	/// use chrono::Duration;
	///
	/// let jwt_auth = JwtAuth::new(b"secret");
	/// let claims = Claims::new(
	///     "user123".to_string(),
	///     "john".to_string(),
	///     Duration::hours(1)
	/// );
	///
	/// let token = jwt_auth.encode(&claims).unwrap();
	/// assert!(!token.is_empty());
	/// ```
	pub fn encode(&self, claims: &Claims) -> reinhardt_core::exception::Result<String> {
		encode(&Header::default(), claims, &self.encoding_key)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}
	/// Decodes a JWT token string into claims.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::{JwtAuth, Claims};
	/// use chrono::Duration;
	///
	/// let jwt_auth = JwtAuth::new(b"secret");
	/// let claims = Claims::new(
	///     "user123".to_string(),
	///     "john".to_string(),
	///     Duration::hours(1)
	/// );
	///
	/// let token = jwt_auth.encode(&claims).unwrap();
	/// let decoded = jwt_auth.decode(&token).unwrap();
	/// assert_eq!(decoded.sub, "user123");
	/// ```
	pub fn decode(&self, token: &str) -> reinhardt_core::exception::Result<Claims> {
		decode::<Claims>(token, &self.decoding_key, &self.validation)
			.map(|data| data.claims)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}
	/// Generates a JWT token for the given user with 24-hour expiration.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::JwtAuth;
	///
	/// let jwt_auth = JwtAuth::new(b"secret");
	/// let token = jwt_auth.generate_token(
	///     "user123".to_string(),
	///     "john_doe".to_string()
	/// ).unwrap();
	///
	/// assert!(!token.is_empty());
	/// assert!(token.contains('.'));
	/// ```
	pub fn generate_token(
		&self,
		user_id: String,
		username: String,
	) -> reinhardt_core::exception::Result<String> {
		let claims = Claims::new(user_id, username, Duration::hours(24));
		self.encode(&claims)
	}
	/// Verifies a JWT token and returns the claims if valid and not expired.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::jwt::JwtAuth;
	///
	/// let jwt_auth = JwtAuth::new(b"secret");
	/// let token = jwt_auth.generate_token(
	///     "user123".to_string(),
	///     "john_doe".to_string()
	/// ).unwrap();
	///
	/// let claims = jwt_auth.verify_token(&token).unwrap();
	/// assert_eq!(claims.sub, "user123");
	/// assert_eq!(claims.username, "john_doe");
	/// ```
	pub fn verify_token(&self, token: &str) -> reinhardt_core::exception::Result<Claims> {
		let claims = self.decode(token)?;

		if claims.is_expired() {
			return Err(reinhardt_core::exception::Error::Authentication(
				"Token expired".to_string(),
			));
		}

		Ok(claims)
	}
}

// Implement DRF-style Authentication trait
#[async_trait::async_trait]
impl Authentication for JwtAuth {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		// Get Authorization header
		let auth_header = request
			.headers
			.get("Authorization")
			.and_then(|h| h.to_str().ok());

		if let Some(header) = auth_header {
			// Check for Bearer token
			if let Some(token) = header.strip_prefix("Bearer ") {
				// Verify and decode token
				match self.verify_token(token) {
					Ok(claims) => {
						// Create user from claims
						return Ok(Some(Box::new(SimpleUser {
							id: Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()),
							username: claims.username.clone(),
							email: format!("{}@example.com", claims.username),
							is_active: true,
							is_admin: false,
							is_staff: false,
							is_superuser: false,
						})));
					}
					Err(_) => {
						return Err(AuthenticationError::InvalidToken);
					}
				}
			}
		}

		Ok(None)
	}
}
