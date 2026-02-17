//! Application lifecycle hooks
//!
//! This module provides lifecycle hooks for applications, allowing applications
//! to perform initialization tasks when they are ready.
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_apps::hooks::{AppReadyHook, ReadyContext};
//! use reinhardt_apps::AppConfig;
//!
//! struct MyAppReadyHook;
//!
//! impl AppReadyHook for MyAppReadyHook {
//!     fn ready(&self, ctx: &ReadyContext) -> Result<(), Box<dyn std::error::Error>> {
//!         println!("App {} is ready!", ctx.config.label);
//!         Ok(())
//!     }
//! }
//! ```

use crate::apps::AppConfig;
use std::error::Error;

/// Context passed to ready hooks
///
/// This structure contains information about the application being initialized
/// and can be extended with additional context in the future.
#[derive(Debug)]
pub struct ReadyContext {
	/// The application configuration
	pub config: AppConfig,
}

impl ReadyContext {
	/// Create a new ready context
	pub fn new(config: AppConfig) -> Self {
		Self { config }
	}

	/// Get the application label
	pub fn app_label(&self) -> &str {
		&self.config.label
	}

	/// Get the application name
	pub fn app_name(&self) -> &str {
		&self.config.name
	}
}

/// Trait for application ready hooks
///
/// Implement this trait to define custom initialization logic that should
/// be executed when an application is ready.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::hooks::{AppReadyHook, ReadyContext};
///
/// struct DatabaseMigrationHook;
///
/// impl AppReadyHook for DatabaseMigrationHook {
///     fn ready(&self, ctx: &ReadyContext) -> Result<(), Box<dyn std::error::Error>> {
///         println!("Running migrations for {}", ctx.app_label());
///         // Run migrations here
///         Ok(())
///     }
/// }
/// ```
pub trait AppReadyHook: Send + Sync {
	/// Called when the application is ready
	///
	/// This method is called after all application configurations have been
	/// loaded and models have been registered.
	///
	/// # Arguments
	///
	/// * `ctx` - Context containing information about the application
	///
	/// # Errors
	///
	/// Returns an error if initialization fails. This will prevent the
	/// application from starting.
	fn ready(&self, ctx: &ReadyContext) -> Result<(), Box<dyn Error>>;
}

/// Registry for application ready hooks
///
/// This allows applications to register hooks that will be executed
/// when the application is ready.
#[derive(Default)]
pub struct HookRegistry {
	hooks: Vec<Box<dyn AppReadyHook>>,
}

impl HookRegistry {
	/// Create a new hook registry
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::HookRegistry;
	///
	/// let registry = HookRegistry::new();
	/// ```
	pub fn new() -> Self {
		Self { hooks: Vec::new() }
	}

	/// Register a ready hook
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::{HookRegistry, AppReadyHook, ReadyContext};
	///
	/// struct MyHook;
	/// impl AppReadyHook for MyHook {
	///     fn ready(&self, _ctx: &ReadyContext) -> Result<(), Box<dyn std::error::Error>> {
	///         Ok(())
	///     }
	/// }
	///
	/// let mut registry = HookRegistry::new();
	/// registry.register(Box::new(MyHook));
	/// ```
	pub fn register(&mut self, hook: Box<dyn AppReadyHook>) {
		self.hooks.push(hook);
	}

	/// Execute all registered hooks for an application
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::{HookRegistry, ReadyContext};
	/// use reinhardt_apps::AppConfig;
	///
	/// let registry = HookRegistry::new();
	/// let config = AppConfig::new("myapp", "myapp");
	/// let ctx = ReadyContext::new(config);
	///
	/// let result = registry.execute(&ctx);
	/// assert!(result.is_ok());
	/// ```
	pub fn execute(&self, ctx: &ReadyContext) -> Result<(), Box<dyn Error>> {
		for hook in &self.hooks {
			hook.ready(ctx)?;
		}
		Ok(())
	}

	/// Get the number of registered hooks
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::HookRegistry;
	///
	/// let registry = HookRegistry::new();
	/// assert_eq!(registry.len(), 0);
	/// ```
	pub fn len(&self) -> usize {
		self.hooks.len()
	}

	/// Check if the registry is empty
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::HookRegistry;
	///
	/// let registry = HookRegistry::new();
	/// assert!(registry.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.hooks.is_empty()
	}

	/// Clear all registered hooks
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::hooks::{HookRegistry, AppReadyHook, ReadyContext};
	///
	/// struct MyHook;
	/// impl AppReadyHook for MyHook {
	///     fn ready(&self, _ctx: &ReadyContext) -> Result<(), Box<dyn std::error::Error>> {
	///         Ok(())
	///     }
	/// }
	///
	/// let mut registry = HookRegistry::new();
	/// registry.register(Box::new(MyHook));
	/// assert_eq!(registry.len(), 1);
	///
	/// registry.clear();
	/// assert_eq!(registry.len(), 0);
	/// ```
	pub fn clear(&mut self) {
		self.hooks.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct TestHook {
		executed: std::sync::Arc<std::sync::atomic::AtomicBool>,
	}

	impl AppReadyHook for TestHook {
		fn ready(&self, _ctx: &ReadyContext) -> Result<(), Box<dyn Error>> {
			self.executed
				.store(true, std::sync::atomic::Ordering::SeqCst);
			Ok(())
		}
	}

	struct FailingHook;

	impl AppReadyHook for FailingHook {
		fn ready(&self, _ctx: &ReadyContext) -> Result<(), Box<dyn Error>> {
			Err("Hook failed".into())
		}
	}

	#[rstest]
	fn test_ready_context_creation() {
		let config = AppConfig::new("myapp", "myapp");
		let ctx = ReadyContext::new(config);

		assert_eq!(ctx.app_label(), "myapp");
		assert_eq!(ctx.app_name(), "myapp");
	}

	#[rstest]
	fn test_hook_registry_new() {
		let registry = HookRegistry::new();
		assert_eq!(registry.len(), 0);
		assert!(registry.is_empty());
	}

	#[rstest]
	fn test_hook_registry_register() {
		let mut registry = HookRegistry::new();
		let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

		let hook = TestHook {
			executed: executed.clone(),
		};
		registry.register(Box::new(hook));

		assert_eq!(registry.len(), 1);
		assert!(!registry.is_empty());
	}

	#[rstest]
	fn test_hook_registry_execute() {
		let mut registry = HookRegistry::new();
		let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

		let hook = TestHook {
			executed: executed.clone(),
		};
		registry.register(Box::new(hook));

		let config = AppConfig::new("testapp", "testapp");
		let ctx = ReadyContext::new(config);

		let result = registry.execute(&ctx);
		assert!(result.is_ok());
		assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[rstest]
	fn test_hook_registry_execute_failure() {
		let mut registry = HookRegistry::new();
		registry.register(Box::new(FailingHook));

		let config = AppConfig::new("testapp", "testapp");
		let ctx = ReadyContext::new(config);

		let result = registry.execute(&ctx);
		assert!(result.is_err());
	}

	#[rstest]
	fn test_hook_registry_multiple_hooks() {
		let mut registry = HookRegistry::new();

		let executed1 = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
		let executed2 = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

		let hook1 = TestHook {
			executed: executed1.clone(),
		};
		let hook2 = TestHook {
			executed: executed2.clone(),
		};

		registry.register(Box::new(hook1));
		registry.register(Box::new(hook2));

		assert_eq!(registry.len(), 2);

		let config = AppConfig::new("testapp", "testapp");
		let ctx = ReadyContext::new(config);

		let result = registry.execute(&ctx);
		assert!(result.is_ok());

		assert!(executed1.load(std::sync::atomic::Ordering::SeqCst));
		assert!(executed2.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[rstest]
	fn test_hook_registry_clear() {
		let mut registry = HookRegistry::new();
		let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

		let hook = TestHook {
			executed: executed.clone(),
		};
		registry.register(Box::new(hook));

		assert_eq!(registry.len(), 1);

		registry.clear();

		assert_eq!(registry.len(), 0);
		assert!(registry.is_empty());
	}

	#[rstest]
	fn test_hook_registry_stop_on_first_failure() {
		let mut registry = HookRegistry::new();

		let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

		// Register a failing hook first
		registry.register(Box::new(FailingHook));

		// Register a hook that should not execute
		let hook = TestHook {
			executed: executed.clone(),
		};
		registry.register(Box::new(hook));

		let config = AppConfig::new("testapp", "testapp");
		let ctx = ReadyContext::new(config);

		let result = registry.execute(&ctx);
		assert!(result.is_err());

		// The second hook should not have been executed
		assert!(!executed.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[rstest]
	fn test_ready_context_with_verbose_name() {
		let config = AppConfig::new("myapp", "myapp").with_verbose_name("My Application");
		let ctx = ReadyContext::new(config);

		assert_eq!(ctx.config.verbose_name, Some("My Application".to_string()));
	}
}
