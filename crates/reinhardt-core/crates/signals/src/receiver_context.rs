//! Receiver context for dependency injection support
//!
//! This module provides the `ReceiverContext` struct that enables dependency injection
//! in signal receivers.
//!
//! # Usage
//!
//! ```rust,no_run
//! use reinhardt_signals::{Signal, ReceiverContext};
//! use reinhardt_di::{InjectionContext, SingletonScope};
//! use std::sync::Arc;
//!
//! // Create a signal and send with DI context
//! let signal = Signal::<User>::new(SignalName::custom("user_created"));
//! let singleton = Arc::new(SingletonScope::new());
//! let di_ctx = Arc::new(InjectionContext::builder(singleton).build());
//!
//! // Send signal with DI context
//! signal.send_with_di_context(user, di_ctx).await?;
//!
//! // In receiver, resolve dependencies from context
//! signal.connect_with_context(|instance, ctx| async move {
//!     let db: Arc<DatabaseConnection> = ctx.resolve().await?;
//!     // Use dependency...
//!     Ok(())
//! });
//! ```

#[cfg(feature = "di")]
use crate::error::SignalError;
#[cfg(feature = "di")]
use reinhardt_di::{Injectable, Injected, InjectionContext};
#[cfg(feature = "di")]
use std::sync::Arc;

/// Context passed to signal receivers, optionally containing DI context
///
/// This struct allows receivers to resolve dependencies when the signal
/// is sent with a DI context via `send_with_di_context()`.
#[derive(Clone)]
pub struct ReceiverContext {
	/// Optional DI context for dependency injection
	#[cfg(feature = "di")]
	di_context: Option<Arc<InjectionContext>>,

	#[cfg(not(feature = "di"))]
	_phantom: std::marker::PhantomData<()>,
}

impl Default for ReceiverContext {
	fn default() -> Self {
		Self::new()
	}
}

impl ReceiverContext {
	/// Create a new empty receiver context (without DI)
	pub fn new() -> Self {
		Self {
			#[cfg(feature = "di")]
			di_context: None,
			#[cfg(not(feature = "di"))]
			_phantom: std::marker::PhantomData,
		}
	}

	/// Create a receiver context with DI context
	#[cfg(feature = "di")]
	pub fn with_di_context(di_context: Arc<InjectionContext>) -> Self {
		Self {
			di_context: Some(di_context),
		}
	}

	/// Get the DI context if available
	#[cfg(feature = "di")]
	pub fn di_context(&self) -> Option<&Arc<InjectionContext>> {
		self.di_context.as_ref()
	}

	/// Resolve a dependency from the DI context with caching
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The DI context is not available (signal not sent with `send_with_di_context`)
	/// - The dependency cannot be resolved
	///
	/// # Examples
	///
	/// ```ignore
	/// signal.connect_with_context(|instance, ctx| async move {
	///     let db: Arc<DatabaseConnection> = ctx.resolve().await?;
	///     Ok(())
	/// });
	/// ```
	#[cfg(feature = "di")]
	pub async fn resolve<T>(&self) -> Result<T, SignalError>
	where
		T: Injectable + Clone + Send + Sync + 'static,
	{
		let di_ctx = self.di_context.as_ref().ok_or_else(|| {
			SignalError::new(
				"DI context not available. Use signal.send_with_di_context() to enable injection",
			)
		})?;

		let injected = Injected::<T>::resolve(di_ctx).await.map_err(|e| {
			SignalError::new(format!(
				"Dependency injection failed for {}: {:?}",
				std::any::type_name::<T>(),
				e
			))
		})?;

		Ok(injected.into_inner())
	}

	/// Resolve a dependency from the DI context without caching
	///
	/// Creates a fresh instance each time, bypassing the cache.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The DI context is not available
	/// - The dependency cannot be resolved
	#[cfg(feature = "di")]
	pub async fn resolve_uncached<T>(&self) -> Result<T, SignalError>
	where
		T: Injectable + Clone + Send + Sync + 'static,
	{
		let di_ctx = self.di_context.as_ref().ok_or_else(|| {
			SignalError::new(
				"DI context not available. Use signal.send_with_di_context() to enable injection",
			)
		})?;

		let injected = Injected::<T>::resolve_uncached(di_ctx).await.map_err(|e| {
			SignalError::new(format!(
				"Dependency injection failed for {}: {:?}",
				std::any::type_name::<T>(),
				e
			))
		})?;

		Ok(injected.into_inner())
	}

	/// Try to resolve a dependency, returning None if DI context is not available
	///
	/// # Examples
	///
	/// ```ignore
	/// signal.connect_with_context(|instance, ctx| async move {
	///     if let Some(cache) = ctx.try_resolve::<CacheService>().await {
	///         // Use cache
	///     } else {
	///         // Fallback without cache
	///     }
	///     Ok(())
	/// });
	/// ```
	#[cfg(feature = "di")]
	pub async fn try_resolve<T>(&self) -> Option<T>
	where
		T: Injectable + Clone + Send + Sync + 'static,
	{
		let di_ctx = self.di_context.as_ref()?;

		Injected::<T>::resolve(di_ctx)
			.await
			.ok()
			.map(|injected| injected.into_inner())
	}

	/// Check if DI context is available
	#[cfg(feature = "di")]
	pub fn has_di_context(&self) -> bool {
		self.di_context.is_some()
	}

	/// Check if DI context is available (always false when di feature is disabled)
	#[cfg(not(feature = "di"))]
	pub fn has_di_context(&self) -> bool {
		false
	}
}

impl std::fmt::Debug for ReceiverContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut debug = f.debug_struct("ReceiverContext");

		#[cfg(feature = "di")]
		debug.field("has_di_context", &self.di_context.is_some());

		#[cfg(not(feature = "di"))]
		debug.field("has_di_context", &false);

		debug.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_receiver_context_new() {
		let ctx = ReceiverContext::new();
		assert!(!ctx.has_di_context());
	}

	#[test]
	fn test_receiver_context_default() {
		let ctx = ReceiverContext::default();
		assert!(!ctx.has_di_context());
	}

	#[test]
	fn test_receiver_context_debug() {
		let ctx = ReceiverContext::new();
		let debug_str = format!("{:?}", ctx);
		assert!(debug_str.contains("ReceiverContext"));
		assert!(debug_str.contains("has_di_context"));
	}
}
