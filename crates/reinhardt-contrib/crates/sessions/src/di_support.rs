//! Dependency Injection support for sessions
//!
//! This module provides dependency injection integration for sessions,
//! allowing sessions to be injected into handlers using the DI system.
//!
//! ## Features
//!
//! - **SessionProvider**: DI provider for session injection
//! - **Injectable trait**: Session DI integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_sessions::di_support::SessionProvider;
//! use reinhardt_sessions::backends::InMemorySessionBackend;
//! use reinhardt_core::di::{Injectable, InjectionContext, SingletonScope};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create session backend and provider
//! let backend = InMemorySessionBackend::new();
//! let provider = SessionProvider::new(backend);
//!
//! // Create injection context
//! let singleton_scope = Arc::new(SingletonScope::new());
//! let ctx = InjectionContext::new(singleton_scope);
//!
//! // Inject session
//! let session = provider.resolve(&ctx).await?;
//! # Ok(())
//! # }
//! ```

use crate::backends::SessionBackend;
use crate::session::Session;
use async_trait::async_trait;
use reinhardt_core::di::{DiError, DiResult, Injectable, InjectionContext};
use std::sync::Arc;

/// Session provider for dependency injection
///
/// Provides session instances through the DI system. The provider creates
/// new sessions using the configured backend.
///
/// ## Example
///
/// ```rust
/// use reinhardt_sessions::di_support::SessionProvider;
/// use reinhardt_sessions::backends::InMemorySessionBackend;
/// use std::sync::Arc;
///
/// let backend = InMemorySessionBackend::new();
/// let provider = SessionProvider::new(backend);
/// ```
#[derive(Clone)]
pub struct SessionProvider<B: SessionBackend> {
	backend: Arc<B>,
}

impl<B: SessionBackend + 'static> SessionProvider<B> {
	/// Create a new session provider with the given backend
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_sessions::di_support::SessionProvider;
	/// use reinhardt_sessions::backends::InMemorySessionBackend;
	///
	/// let backend = InMemorySessionBackend::new();
	/// let provider = SessionProvider::new(backend);
	/// ```
	pub fn new(backend: B) -> Self {
		Self {
			backend: Arc::new(backend),
		}
	}

	/// Create a session provider from an existing Arc-wrapped backend
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_sessions::di_support::SessionProvider;
	/// use reinhardt_sessions::backends::InMemorySessionBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(InMemorySessionBackend::new());
	/// let provider = SessionProvider::from_arc(backend);
	/// ```
	pub fn from_arc(backend: Arc<B>) -> Self {
		Self { backend }
	}

	/// Get the backend used by this provider
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_sessions::di_support::SessionProvider;
	/// use reinhardt_sessions::backends::InMemorySessionBackend;
	///
	/// let backend = InMemorySessionBackend::new();
	/// let provider = SessionProvider::new(backend);
	/// let _backend_ref = provider.backend();
	/// ```
	pub fn backend(&self) -> &Arc<B> {
		&self.backend
	}

	/// Resolve a session from the injection context
	///
	/// This method checks the request scope for an existing session.
	/// If not found, it creates a new session.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_sessions::di_support::SessionProvider;
	/// use reinhardt_sessions::backends::InMemorySessionBackend;
	/// use reinhardt_core::di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = InMemorySessionBackend::new();
	/// let provider = SessionProvider::new(backend);
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton_scope);
	///
	/// let session = provider.resolve(&ctx).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn resolve(&self, ctx: &InjectionContext) -> DiResult<Arc<Session<B>>> {
		// Try to get from request scope first (cached)
		if let Some(cached) = ctx.get_request::<Session<B>>() {
			return Ok(cached);
		}

		// Create new session
		let session = Session::new((*self.backend).clone());
		let session_arc = Arc::new(session);

		// Cache in request scope
		ctx.set_request((*session_arc).clone());

		Ok(session_arc)
	}
}

/// Injectable implementation for Session
///
/// Allows sessions to be automatically injected using `Depends<Session<B>>`.
#[async_trait]
impl<B: SessionBackend + 'static> Injectable for Session<B> {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Try to get from request scope first (cached)
		if let Some(cached) = ctx.get_request::<Self>() {
			return Ok(Arc::try_unwrap(cached).unwrap_or_else(|arc| (*arc).clone()));
		}

		// Try to get provider from singleton scope
		if let Some(provider) = ctx.get_singleton::<SessionProvider<B>>() {
			let session_arc = provider.resolve(ctx).await?;
			return Ok(Arc::try_unwrap(session_arc).unwrap_or_else(|arc| (*arc).clone()));
		}

		// No provider configured
		Err(DiError::NotFound(
			"SessionProvider not found in singleton scope".to_string(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::InMemorySessionBackend;
	use reinhardt_core::di::SingletonScope;

	#[tokio::test]
	async fn test_session_provider_new() {
		let backend = InMemorySessionBackend::new();
		let _provider = SessionProvider::new(backend);
	}

	#[tokio::test]
	async fn test_session_provider_from_arc() {
		let backend = Arc::new(InMemorySessionBackend::new());
		let _provider = SessionProvider::from_arc(backend);
	}

	#[tokio::test]
	async fn test_session_provider_backend() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend);
		let _backend_ref = provider.backend();
	}

	#[tokio::test]
	async fn test_session_provider_resolve_creates_new_session() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend);

		let singleton_scope = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton_scope);

		let session = provider.resolve(&ctx).await.unwrap();
		assert!(session.session_key().is_none());
	}

	#[tokio::test]
	async fn test_session_provider_resolve_caches_in_request_scope() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend);

		let singleton_scope = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton_scope);

		// First resolve
		let session1 = provider.resolve(&ctx).await.unwrap();
		let mut session1_mut = Arc::try_unwrap(session1).unwrap_or_else(|arc| (*arc).clone());
		let key1 = session1_mut.get_or_create_key().to_string();

		// Cache the session
		ctx.set_request(session1_mut);

		// Second resolve should return cached session
		let session2 = provider.resolve(&ctx).await.unwrap();
		let mut session2_mut = Arc::try_unwrap(session2).unwrap_or_else(|arc| (*arc).clone());
		let key2 = session2_mut.get_or_create_key().to_string();

		assert_eq!(key1, key2);
	}

	#[tokio::test]
	async fn test_session_injectable_with_provider_in_singleton() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend);

		let singleton_scope = Arc::new(SingletonScope::new());
		singleton_scope.set(provider);

		let ctx = InjectionContext::new(singleton_scope);

		// Inject session
		let session = Session::<InMemorySessionBackend>::inject(&ctx)
			.await
			.unwrap();
		assert!(session.session_key().is_none());
	}

	#[tokio::test]
	async fn test_session_injectable_without_provider_fails() {
		let singleton_scope = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton_scope);

		// Should fail because no provider is configured
		let result = Session::<InMemorySessionBackend>::inject(&ctx).await;
		assert!(result.is_err());
		assert!(matches!(result, Err(DiError::NotFound(_))));
	}

	#[tokio::test]
	async fn test_session_injectable_returns_cached_session() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend.clone());

		let singleton_scope = Arc::new(SingletonScope::new());
		singleton_scope.set(provider);

		let ctx = InjectionContext::new(singleton_scope);

		// First injection
		let mut session1 = Session::<InMemorySessionBackend>::inject(&ctx)
			.await
			.unwrap();
		let key1 = session1.get_or_create_key().to_string();

		// Cache the session
		ctx.set_request(session1);

		// Second injection should return the same session
		let mut session2 = Session::<InMemorySessionBackend>::inject(&ctx)
			.await
			.unwrap();
		let key2 = session2.get_or_create_key().to_string();

		assert_eq!(key1, key2);
	}

	#[tokio::test]
	async fn test_session_provider_multiple_backends() {
		let backend1 = InMemorySessionBackend::new();
		let backend2 = InMemorySessionBackend::new();

		let provider1 = SessionProvider::new(backend1);
		let provider2 = SessionProvider::new(backend2);

		let singleton_scope = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton_scope);

		// Both providers should work independently
		let _session1 = provider1.resolve(&ctx).await.unwrap();
		let _session2 = provider2.resolve(&ctx).await.unwrap();
	}

	#[tokio::test]
	async fn test_session_provider_resolve_preserves_session_data() {
		let backend = InMemorySessionBackend::new();
		let provider = SessionProvider::new(backend);

		let singleton_scope = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton_scope);

		// Resolve and modify session
		let session = provider.resolve(&ctx).await.unwrap();
		let mut session_mut = Arc::try_unwrap(session).unwrap_or_else(|arc| (*arc).clone());
		session_mut.set("test_key", "test_value").unwrap();

		// Cache the modified session
		ctx.set_request(session_mut.clone());

		// Resolve again
		let session2 = provider.resolve(&ctx).await.unwrap();
		let mut session2_mut = Arc::try_unwrap(session2).unwrap_or_else(|arc| (*arc).clone());
		let value: String = session2_mut.get("test_key").unwrap().unwrap();
		assert_eq!(value, "test_value");
	}
}
