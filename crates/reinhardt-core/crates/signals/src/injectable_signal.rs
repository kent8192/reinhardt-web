//! Dependency injection extension for Signals
//!
//! This module provides the `InjectableSignal` extension trait that enables
//! dependency injection in signal receivers without modifying the core Signal type.
//!
//! # Usage
//!
//! ```rust,no_run
//! use reinhardt_signals::{Signal, SignalName, InjectableSignal, ReceiverContext};
//! use reinhardt_di::{InjectionContext, SingletonScope};
//! use std::sync::Arc;
//!
//! // Create a signal
//! let signal = Signal::<User>::new(SignalName::custom("user_created"));
//!
//! // Connect a receiver with context
//! signal.connect_with_context(|instance, ctx| async move {
//!     let db: Arc<DatabaseConnection> = ctx.resolve().await?;
//!     println!("User {:?} saved, using db connection", instance);
//!     Ok(())
//! });
//!
//! // Send signal with DI context
//! let singleton = Arc::new(SingletonScope::new());
//! let di_ctx = Arc::new(InjectionContext::builder(singleton).build());
//! signal.send_with_di_context(user, di_ctx).await?;
//! ```

use crate::error::SignalError;
use crate::receiver_context::ReceiverContext;
use crate::signal::Signal;
use async_trait::async_trait;
use std::future::Future;
use std::sync::Arc;

#[cfg(feature = "di")]
use reinhardt_di::InjectionContext;

/// Extension trait for Signals that enables dependency injection
///
/// This trait is automatically implemented for all Signal types.
/// It provides helper methods to send signals with DI context and
/// connect receivers that can resolve dependencies.
#[async_trait]
pub trait InjectableSignal<T: Send + Sync + 'static> {
	/// Send signal with a DI context for dependency injection in receivers
	///
	/// Receivers connected via `connect_with_context` will be able to resolve
	/// dependencies from the provided injection context.
	///
	/// # Arguments
	/// * `instance` - The instance to send
	/// * `di_context` - The injection context for dependency resolution
	///
	/// # Examples
	///
	/// ```ignore
	/// let singleton = Arc::new(SingletonScope::new());
	/// let di_ctx = Arc::new(InjectionContext::builder(singleton).build());
	/// signal.send_with_di_context(user, di_ctx).await?;
	/// ```
	#[cfg(feature = "di")]
	async fn send_with_di_context(
		&self,
		instance: T,
		di_context: Arc<InjectionContext>,
	) -> Result<(), SignalError>;

	/// Connect a receiver function that receives a ReceiverContext
	///
	/// The receiver context allows the receiver to resolve dependencies
	/// when the signal is sent via `send_with_di_context`.
	///
	/// # Examples
	///
	/// ```ignore
	/// signal.connect_with_context(|instance, ctx| async move {
	///     // Resolve dependencies from context
	///     let db: Arc<DatabaseConnection> = ctx.resolve().await?;
	///     println!("Processing {:?} with db connection", instance);
	///     Ok(())
	/// });
	/// ```
	fn connect_with_context<F, Fut>(&self, receiver: F)
	where
		F: Fn(Arc<T>, ReceiverContext) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<(), SignalError>> + Send + 'static;
}

// Thread-local storage for DI context during signal dispatch
#[cfg(feature = "di")]
std::thread_local! {
	static CURRENT_DI_CONTEXT: std::cell::RefCell<Option<Arc<InjectionContext>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "di")]
fn set_current_di_context(ctx: Option<Arc<InjectionContext>>) {
	CURRENT_DI_CONTEXT.with(|c| {
		*c.borrow_mut() = ctx;
	});
}

#[cfg(feature = "di")]
fn get_current_di_context() -> Option<Arc<InjectionContext>> {
	CURRENT_DI_CONTEXT.with(|c| c.borrow().clone())
}

#[async_trait]
impl<T: Send + Sync + 'static> InjectableSignal<T> for Signal<T> {
	#[cfg(feature = "di")]
	async fn send_with_di_context(
		&self,
		instance: T,
		di_context: Arc<InjectionContext>,
	) -> Result<(), SignalError> {
		// Store the DI context in thread-local storage for receivers to access
		set_current_di_context(Some(di_context));

		// Send the signal using the standard method
		let result = self.send(instance).await;

		// Clear the DI context after sending
		set_current_di_context(None);

		result
	}

	fn connect_with_context<F, Fut>(&self, receiver: F)
	where
		F: Fn(Arc<T>, ReceiverContext) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
	{
		// Wrap the context-aware receiver to work with the standard connect API
		self.connect(move |instance: Arc<T>| {
			// Create context with current DI context if available
			#[cfg(feature = "di")]
			let ctx = {
				match get_current_di_context() {
					Some(di_ctx) => ReceiverContext::with_di_context(di_ctx),
					None => ReceiverContext::new(),
				}
			};

			#[cfg(not(feature = "di"))]
			let ctx = ReceiverContext::new();

			receiver(instance, ctx)
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::SignalName;

	#[allow(dead_code)]
	#[derive(Debug, Clone)]
	struct TestModel {
		id: i32,
		name: String,
	}

	#[tokio::test]
	async fn test_connect_with_context() {
		let signal = Signal::new(SignalName::custom("test_context"));

		let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let counter_clone = counter.clone();

		signal.connect_with_context(move |_instance: Arc<TestModel>, ctx: ReceiverContext| {
			let counter = counter_clone.clone();
			async move {
				// Context should be available but without DI context
				assert!(!ctx.has_di_context());
				counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				Ok(())
			}
		});

		let model = TestModel {
			id: 1,
			name: "Test".to_string(),
		};

		// Send without DI context (using standard send)
		signal.send(model).await.unwrap();

		assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
	}

	#[cfg(feature = "di")]
	#[tokio::test]
	async fn test_send_with_di_context() {
		use reinhardt_di::SingletonScope;

		let signal = Signal::new(SignalName::custom("test_di_context"));

		let had_di_context = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
		let had_di_context_clone = had_di_context.clone();

		signal.connect_with_context(move |_instance: Arc<TestModel>, ctx: ReceiverContext| {
			let had_di_context = had_di_context_clone.clone();
			async move {
				// When sent with DI context, it should be available
				had_di_context.store(ctx.has_di_context(), std::sync::atomic::Ordering::SeqCst);
				Ok(())
			}
		});

		let model = TestModel {
			id: 1,
			name: "Test".to_string(),
		};

		let singleton = SingletonScope::new();
		let di_ctx = Arc::new(InjectionContext::builder(singleton).build());
		signal.send_with_di_context(model, di_ctx).await.unwrap();

		assert!(had_di_context.load(std::sync::atomic::Ordering::SeqCst));
	}
}
