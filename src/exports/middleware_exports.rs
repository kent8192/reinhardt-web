//! Middleware re-exports.

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::AuthenticationMiddleware;

#[cfg(feature = "middleware-auth-jwt")]
pub use reinhardt_middleware::JwtAuthMiddleware;

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::{CookieSessionAuthMiddleware, CookieSessionConfig};

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::session::{
	OptionalSessionValue, SessionAuthExt, SessionKey, SessionValue, SessionValueNamed,
	USER_ID_SESSION_KEY, UserIdKey,
};

#[cfg(all(feature = "session-redis", feature = "middleware"))]
pub use reinhardt_middleware::RedisSessionBackend;

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::OriginGuardMiddleware;

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::{PersistentRemoteUserMiddleware, RemoteUserMiddleware};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{LoginRequiredConfig, LoginRequiredMiddleware};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::LoggingMiddleware;

#[cfg(feature = "middleware-cors")]
pub use reinhardt_middleware::CorsMiddleware;

#[cfg(feature = "middleware-security")]
pub use reinhardt_middleware::SecurityMiddleware;

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{CspConfig, CspMiddleware, CspNonce};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{XFrameOptions, XFrameOptionsMiddleware};

#[cfg(feature = "middleware")]
pub use reinhardt_middleware::CacheMiddleware;
