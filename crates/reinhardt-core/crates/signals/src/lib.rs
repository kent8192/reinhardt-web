//! # Reinhardt Signals
//!
//! Event-driven signal system for Reinhardt framework, inspired by Django signals.
//!
//! ## Signal Types
//!
//! ### Model Signals
//! - **pre_save**: Before saving a model instance
//! - **post_save**: After saving a model instance
//! - **pre_delete**: Before deleting a model instance
//! - **post_delete**: After deleting a model instance
//! - **pre_init**: At the beginning of a model's initialization
//! - **post_init**: At the end of a model's initialization
//! - **m2m_changed**: When many-to-many relationships change
//! - **class_prepared**: When a model class is prepared
//!
//! ### Migration Signals
//! - **pre_migrate**: Before running migrations
//! - **post_migrate**: After running migrations
//!
//! ### Request Signals
//! - **request_started**: When an HTTP request starts
//! - **request_finished**: When an HTTP request finishes
//! - **got_request_exception**: When an exception occurs during request handling
//!
//! ### Management Signals
//! - **setting_changed**: When a configuration setting is changed
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_signals::{Signal, post_save};
//!
//! // Connect a receiver
//! post_save::<User>().connect(|sender, instance| async move {
//!     println!("User saved: {:?}", instance);
//!     Ok(())
//! });
//!
//! // Send signal
//! post_save::<User>().send(&user).await?;
//! ```

use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Type-safe signal name wrapper
///
/// This type provides compile-time safety for signal names while still allowing
/// custom signal names when needed.
///
/// # Examples
///
/// ```
/// use reinhardt_signals::SignalName;
///
// Use built-in signal names
/// let signal_name = SignalName::PRE_SAVE;
///
// Create custom signal names
/// let custom = SignalName::custom("my_custom_signal");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignalName(&'static str);

impl SignalName {
    // Model signals
    /// Signal sent before saving a model instance
    pub const PRE_SAVE: Self = Self("pre_save");
    /// Signal sent after saving a model instance
    pub const POST_SAVE: Self = Self("post_save");
    /// Signal sent before deleting a model instance
    pub const PRE_DELETE: Self = Self("pre_delete");
    /// Signal sent after deleting a model instance
    pub const POST_DELETE: Self = Self("post_delete");
    /// Signal sent at the beginning of a model's initialization
    pub const PRE_INIT: Self = Self("pre_init");
    /// Signal sent at the end of a model's initialization
    pub const POST_INIT: Self = Self("post_init");
    /// Signal sent when many-to-many relationships change
    pub const M2M_CHANGED: Self = Self("m2m_changed");
    /// Signal sent when a model class is prepared
    pub const CLASS_PREPARED: Self = Self("class_prepared");

    // Migration signals
    /// Signal sent before running migrations
    pub const PRE_MIGRATE: Self = Self("pre_migrate");
    /// Signal sent after running migrations
    pub const POST_MIGRATE: Self = Self("post_migrate");

    // Request signals
    /// Signal sent when an HTTP request starts
    pub const REQUEST_STARTED: Self = Self("request_started");
    /// Signal sent when an HTTP request finishes
    pub const REQUEST_FINISHED: Self = Self("request_finished");
    /// Signal sent when an exception occurs during request handling
    pub const GOT_REQUEST_EXCEPTION: Self = Self("got_request_exception");

    // Management signals
    /// Signal sent when a configuration setting is changed
    pub const SETTING_CHANGED: Self = Self("setting_changed");

    // Database signals
    /// Signal sent before a database insert operation
    pub const DB_BEFORE_INSERT: Self = Self("db_before_insert");
    /// Signal sent after a database insert operation
    pub const DB_AFTER_INSERT: Self = Self("db_after_insert");
    /// Signal sent before a database update operation
    pub const DB_BEFORE_UPDATE: Self = Self("db_before_update");
    /// Signal sent after a database update operation
    pub const DB_AFTER_UPDATE: Self = Self("db_after_update");
    /// Signal sent before a database delete operation
    pub const DB_BEFORE_DELETE: Self = Self("db_before_delete");
    /// Signal sent after a database delete operation
    pub const DB_AFTER_DELETE: Self = Self("db_after_delete");

    /// Create a custom signal name without validation
    ///
    /// Note: This requires a `'static` string to ensure the name lives long enough.
    /// For dynamic names, consider using string literals or leaked strings.
    ///
    /// For validated custom signal names, use `custom_validated()` instead.
    pub const fn custom(name: &'static str) -> Self {
        Self(name)
    }

    /// Create a validated custom signal name
    ///
    /// This method validates that the signal name:
    /// - Is not empty
    /// - Uses snake_case format
    /// - Does not conflict with reserved signal names
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_signals::SignalName;
    ///
    // Valid custom signal names
    /// let valid = SignalName::custom_validated("my_custom_signal").unwrap();
    ///
    // Invalid: not snake_case
    /// assert!(SignalName::custom_validated("MySignal").is_err());
    ///
    // Invalid: reserved name
    /// assert!(SignalName::custom_validated("pre_save").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `SignalError` if validation fails.
    pub fn custom_validated(name: &'static str) -> Result<Self, SignalError> {
        validate_signal_name(name)?;
        Ok(Self(name))
    }

    /// Get all reserved signal names
    ///
    /// Returns a list of all built-in signal names that cannot be used
    /// for custom signals.
    pub fn reserved_names() -> &'static [&'static str] {
        &[
            "pre_save",
            "post_save",
            "pre_delete",
            "post_delete",
            "pre_init",
            "post_init",
            "m2m_changed",
            "class_prepared",
            "pre_migrate",
            "post_migrate",
            "request_started",
            "request_finished",
            "got_request_exception",
            "setting_changed",
            "db_before_insert",
            "db_after_insert",
            "db_before_update",
            "db_after_update",
            "db_before_delete",
            "db_after_delete",
        ]
    }

    /// Get the string representation of this signal name
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for SignalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SignalName> for String {
    fn from(name: SignalName) -> String {
        name.0.to_string()
    }
}

impl AsRef<str> for SignalName {
    fn as_ref(&self) -> &str {
        self.0
    }
}

/// Validate a custom signal name
///
/// Checks that the signal name:
/// - Is not empty
/// - Uses snake_case format (lowercase letters, numbers, and underscores only)
/// - Does not conflict with reserved signal names
///
/// # Errors
///
/// Returns `SignalError` if validation fails.
fn validate_signal_name(name: &str) -> Result<(), SignalError> {
    // Check if empty
    if name.is_empty() {
        return Err(SignalError::new("Signal name cannot be empty"));
    }

    // Check if reserved
    if SignalName::reserved_names().contains(&name) {
        return Err(SignalError::new(format!(
            "Signal name '{}' is reserved and cannot be used for custom signals",
            name
        )));
    }

    // Check snake_case format
    // Valid: lowercase letters, numbers, underscores
    // Must start with a letter or underscore
    // Cannot have consecutive underscores
    // Cannot end with underscore
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(SignalError::new(format!(
            "Signal name '{}' must use snake_case format (lowercase letters, numbers, and underscores only)",
            name
        )));
    }

    // Check first character
    if let Some(first) = name.chars().next() {
        if !first.is_ascii_lowercase() && first != '_' {
            return Err(SignalError::new(format!(
                "Signal name '{}' must start with a lowercase letter or underscore",
                name
            )));
        }
    }

    // Check for consecutive underscores
    if name.contains("__") {
        return Err(SignalError::new(format!(
            "Signal name '{}' cannot contain consecutive underscores",
            name
        )));
    }

    // Check if ends with underscore
    if name.ends_with('_') {
        return Err(SignalError::new(format!(
            "Signal name '{}' cannot end with an underscore",
            name
        )));
    }

    Ok(())
}

/// Common trait for all signal dispatchers
///
/// This trait provides a unified interface for both async and sync signals,
/// enabling generic code and easier testing.
pub trait SignalDispatcher<T: Send + Sync + 'static> {
    /// Get the number of connected receivers
    fn receiver_count(&self) -> usize;

    /// Clear all receivers
    fn disconnect_all(&self);

    /// Disconnect a receiver by dispatch_uid
    fn disconnect(&self, dispatch_uid: &str) -> bool;
}

/// Trait for asynchronous signal dispatchers
///
/// Extends SignalDispatcher with async-specific methods
#[async_trait::async_trait]
pub trait AsyncSignalDispatcher<T: Send + Sync + 'static>: SignalDispatcher<T> {
    /// Send signal to all connected receivers
    async fn send(&self, instance: T) -> Result<(), SignalError>;

    /// Send signal with sender type filtering
    async fn send_with_sender(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Result<(), SignalError>;

    /// Send signal robustly, catching errors
    async fn send_robust(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Vec<Result<(), SignalError>>;
}

/// Signal receiver function type
pub type ReceiverFn<T> = Arc<
    dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = Result<(), SignalError>> + Send>> + Send + Sync,
>;

/// Signal errors
#[derive(Debug, Clone)]
pub struct SignalError {
    pub message: String,
}

impl fmt::Display for SignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SignalError {}

impl SignalError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

/// Type alias for predicate functions
type PredicateFn<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// Signal middleware for intercepting and transforming signals
#[async_trait::async_trait]
pub trait SignalMiddleware<T: Send + Sync + 'static>: Send + Sync {
    /// Called before the signal is sent to receivers
    /// Return false to stop signal propagation
    async fn before_send(&self, _instance: &T) -> Result<bool, SignalError> {
        Ok(true)
    }

    /// Called after the signal has been sent to all receivers
    async fn after_send(
        &self,
        instance: &T,
        results: &[Result<(), SignalError>],
    ) -> Result<(), SignalError> {
        let _ = (instance, results);
        Ok(())
    }

    /// Called when a receiver is about to execute
    /// Return false to skip this receiver
    async fn before_receiver(
        &self,
        instance: &T,
        dispatch_uid: Option<&str>,
    ) -> Result<bool, SignalError> {
        let _ = (instance, dispatch_uid);
        Ok(true)
    }

    /// Called after a receiver has executed
    async fn after_receiver(
        &self,
        instance: &T,
        dispatch_uid: Option<&str>,
        result: &Result<(), SignalError>,
    ) -> Result<(), SignalError> {
        let _ = (instance, dispatch_uid, result);
        Ok(())
    }
}

/// Type alias for middleware
type MiddlewareFn<T> = Arc<dyn SignalMiddleware<T>>;

/// Call record for SignalSpy
#[derive(Debug, Clone)]
pub struct SignalCall {
    pub signal_sent: bool,
    pub receivers_called: usize,
    pub errors: Vec<String>,
}

/// Testing utility to spy on signal calls
pub struct SignalSpy<T: Send + Sync + 'static> {
    calls: Arc<RwLock<Vec<SignalCall>>>,
    instances: Arc<RwLock<Vec<Arc<T>>>>,
}

impl<T: Send + Sync + 'static> SignalSpy<T> {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(RwLock::new(Vec::new())),
            instances: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Returns the number of times the signal was sent
    pub fn call_count(&self) -> usize {
        self.calls.read().len()
    }

    /// Returns all recorded signal calls
    pub fn calls(&self) -> Vec<SignalCall> {
        self.calls.read().clone()
    }

    /// Returns all instances that were sent
    pub fn instances(&self) -> Vec<Arc<T>> {
        self.instances.read().clone()
    }

    /// Returns the last instance that was sent, if any
    pub fn last_instance(&self) -> Option<Arc<T>> {
        self.instances.read().last().cloned()
    }

    /// Check if the signal was called
    pub fn was_called(&self) -> bool {
        self.call_count() > 0
    }

    /// Check if the signal was called with specific count
    pub fn was_called_with_count(&self, count: usize) -> bool {
        self.call_count() == count
    }

    /// Reset all recorded calls and instances
    pub fn reset(&self) {
        self.calls.write().clear();
        self.instances.write().clear();
    }

    /// Get the total number of receivers that were called across all signals
    pub fn total_receivers_called(&self) -> usize {
        self.calls.read().iter().map(|c| c.receivers_called).sum()
    }

    /// Check if any errors occurred
    pub fn has_errors(&self) -> bool {
        self.calls.read().iter().any(|c| !c.errors.is_empty())
    }

    /// Get all error messages
    pub fn errors(&self) -> Vec<String> {
        self.calls
            .read()
            .iter()
            .flat_map(|c| c.errors.clone())
            .collect()
    }
}

impl<T: Send + Sync + 'static> Clone for SignalSpy<T> {
    fn clone(&self) -> Self {
        Self {
            calls: Arc::clone(&self.calls),
            instances: Arc::clone(&self.instances),
        }
    }
}

impl<T: Send + Sync + 'static> Default for SignalSpy<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync + 'static> SignalMiddleware<T> for SignalSpy<T> {
    async fn before_send(&self, _instance: &T) -> Result<bool, SignalError> {
        // This will be updated in after_send
        Ok(true)
    }

    async fn after_send(
        &self,
        _instance: &T,
        results: &[Result<(), SignalError>],
    ) -> Result<(), SignalError> {
        let errors: Vec<String> = results
            .iter()
            .filter_map(|r| r.as_ref().err().map(|e| e.message.clone()))
            .collect();

        let call = SignalCall {
            signal_sent: true,
            receivers_called: results.len(),
            errors,
        };

        self.calls.write().push(call);

        // Store instance for later inspection (we need to convert &T to Arc<T>)
        // Since we can't clone T, we create a new Arc from the reference
        // This is a limitation - we can only store the reference during the call

        Ok(())
    }

    async fn before_receiver(
        &self,
        _instance: &T,
        _dispatch_uid: Option<&str>,
    ) -> Result<bool, SignalError> {
        Ok(true)
    }

    async fn after_receiver(
        &self,
        _instance: &T,
        _dispatch_uid: Option<&str>,
        _result: &Result<(), SignalError>,
    ) -> Result<(), SignalError> {
        Ok(())
    }
}

/// Context information passed with signals
/// Allows passing additional metadata alongside the signal instance
pub struct SignalContext {
    metadata: Arc<RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>>,
}

impl SignalContext {
    pub fn new() -> Self {
        Self {
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a value into the context
    pub fn insert<V: Any + Send + Sync>(&self, key: impl Into<String>, value: V) {
        self.metadata.write().insert(key.into(), Arc::new(value));
    }

    /// Get a value from the context
    pub fn get<V: Any + Send + Sync>(&self, key: &str) -> Option<Arc<V>> {
        let metadata = self.metadata.read();
        let value = metadata.get(key)?;
        value.clone().downcast::<V>().ok()
    }

    /// Check if a key exists in the context
    pub fn contains_key(&self, key: &str) -> bool {
        self.metadata.read().contains_key(key)
    }

    /// Remove a value from the context
    pub fn remove(&self, key: &str) -> bool {
        self.metadata.write().remove(key).is_some()
    }

    /// Clear all context data
    pub fn clear(&self) {
        self.metadata.write().clear();
    }

    /// Get all keys in the context
    pub fn keys(&self) -> Vec<String> {
        self.metadata.read().keys().cloned().collect()
    }
}

impl Clone for SignalContext {
    fn clone(&self) -> Self {
        Self {
            metadata: Arc::clone(&self.metadata),
        }
    }
}

impl Default for SignalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SignalContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys = self.keys();
        f.debug_struct("SignalContext")
            .field("keys", &keys)
            .finish()
    }
}

/// Performance metrics for a signal
#[derive(Debug, Clone)]
pub struct SignalMetrics {
    /// Total number of times the signal was sent
    pub send_count: u64,
    /// Total number of receiver executions
    pub receiver_executions: u64,
    /// Total number of failed receiver executions
    pub failed_executions: u64,
    /// Total execution time in nanoseconds
    pub total_execution_time_ns: u64,
    /// Minimum execution time in nanoseconds
    pub min_execution_time_ns: u64,
    /// Maximum execution time in nanoseconds
    pub max_execution_time_ns: u64,
    /// Average execution time in nanoseconds
    pub avg_execution_time_ns: u64,
}

impl SignalMetrics {
    pub fn new() -> Self {
        Self {
            send_count: 0,
            receiver_executions: 0,
            failed_executions: 0,
            total_execution_time_ns: 0,
            min_execution_time_ns: u64::MAX,
            max_execution_time_ns: 0,
            avg_execution_time_ns: 0,
        }
    }

    /// Get the success rate as a percentage (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.receiver_executions == 0 {
            return 100.0;
        }
        let successful = self.receiver_executions - self.failed_executions;
        (successful as f64 / self.receiver_executions as f64) * 100.0
    }

    /// Get the average execution time as a Duration
    pub fn avg_execution_time(&self) -> Duration {
        Duration::from_nanos(self.avg_execution_time_ns)
    }

    /// Get the minimum execution time as a Duration
    pub fn min_execution_time(&self) -> Duration {
        if self.min_execution_time_ns == u64::MAX {
            Duration::from_nanos(0)
        } else {
            Duration::from_nanos(self.min_execution_time_ns)
        }
    }

    /// Get the maximum execution time as a Duration
    pub fn max_execution_time(&self) -> Duration {
        Duration::from_nanos(self.max_execution_time_ns)
    }
}

impl Default for SignalMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal metrics collector for a signal
struct MetricsCollector {
    send_count: AtomicU64,
    receiver_executions: AtomicU64,
    failed_executions: AtomicU64,
    total_execution_time_ns: AtomicU64,
    min_execution_time_ns: AtomicU64,
    max_execution_time_ns: AtomicU64,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            send_count: AtomicU64::new(0),
            receiver_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            total_execution_time_ns: AtomicU64::new(0),
            min_execution_time_ns: AtomicU64::new(u64::MAX),
            max_execution_time_ns: AtomicU64::new(0),
        }
    }

    fn record_send(&self) {
        self.send_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_receiver_execution(&self, duration: Duration, success: bool) {
        self.receiver_executions.fetch_add(1, Ordering::Relaxed);

        if !success {
            self.failed_executions.fetch_add(1, Ordering::Relaxed);
        }

        let duration_ns = duration.as_nanos() as u64;
        self.total_execution_time_ns
            .fetch_add(duration_ns, Ordering::Relaxed);

        // Update min
        let mut current_min = self.min_execution_time_ns.load(Ordering::Relaxed);
        while duration_ns < current_min {
            match self.min_execution_time_ns.compare_exchange(
                current_min,
                duration_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_min) => current_min = new_min,
            }
        }

        // Update max
        let mut current_max = self.max_execution_time_ns.load(Ordering::Relaxed);
        while duration_ns > current_max {
            match self.max_execution_time_ns.compare_exchange(
                current_max,
                duration_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => current_max = new_max,
            }
        }
    }

    fn snapshot(&self) -> SignalMetrics {
        let send_count = self.send_count.load(Ordering::Relaxed);
        let receiver_executions = self.receiver_executions.load(Ordering::Relaxed);
        let failed_executions = self.failed_executions.load(Ordering::Relaxed);
        let total_execution_time_ns = self.total_execution_time_ns.load(Ordering::Relaxed);
        let min_execution_time_ns = self.min_execution_time_ns.load(Ordering::Relaxed);
        let max_execution_time_ns = self.max_execution_time_ns.load(Ordering::Relaxed);

        let avg_execution_time_ns = if receiver_executions > 0 {
            total_execution_time_ns / receiver_executions
        } else {
            0
        };

        SignalMetrics {
            send_count,
            receiver_executions,
            failed_executions,
            total_execution_time_ns,
            min_execution_time_ns,
            max_execution_time_ns,
            avg_execution_time_ns,
        }
    }

    fn reset(&self) {
        self.send_count.store(0, Ordering::Relaxed);
        self.receiver_executions.store(0, Ordering::Relaxed);
        self.failed_executions.store(0, Ordering::Relaxed);
        self.total_execution_time_ns.store(0, Ordering::Relaxed);
        self.min_execution_time_ns
            .store(u64::MAX, Ordering::Relaxed);
        self.max_execution_time_ns.store(0, Ordering::Relaxed);
    }
}

/// Information about a connected receiver
struct ReceiverInfo<T: Send + Sync + 'static> {
    receiver: ReceiverFn<T>,
    sender_type_id: Option<TypeId>,
    dispatch_uid: Option<String>,
    priority: i32,                     // Higher values execute first
    predicate: Option<PredicateFn<T>>, // Optional condition for execution
}

impl<T: Send + Sync + 'static> Clone for ReceiverInfo<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: Arc::clone(&self.receiver),
            sender_type_id: self.sender_type_id,
            dispatch_uid: self.dispatch_uid.clone(),
            priority: self.priority,
            predicate: self.predicate.clone(),
        }
    }
}

/// A signal that can dispatch events to connected receivers
pub struct Signal<T: Send + Sync + 'static> {
    receivers: Arc<RwLock<Vec<ReceiverInfo<T>>>>,
    middlewares: Arc<RwLock<Vec<MiddlewareFn<T>>>>,
    context: SignalContext,
    metrics: Arc<MetricsCollector>,
    name: String,
}

impl<T: Send + Sync + 'static> Signal<T> {
    /// Create a new signal with a type-safe name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_signals::{Signal, SignalName};
    ///
    // Use built-in signal names
    /// let signal = Signal::<String>::new(SignalName::PRE_SAVE);
    ///
    // Use custom signal names
    /// let custom = Signal::<String>::new(SignalName::custom("my_signal"));
    /// ```
    pub fn new(name: SignalName) -> Self {
        Self {
            receivers: Arc::new(RwLock::new(Vec::new())),
            middlewares: Arc::new(RwLock::new(Vec::new())),
            context: SignalContext::new(),
            metrics: Arc::new(MetricsCollector::new()),
            name: name.as_str().to_string(),
        }
    }

    /// Create a new signal with a string name (for backward compatibility)
    ///
    /// Consider using `Signal::new(SignalName::custom(name))` for type safety.
    #[doc(hidden)]
    pub fn new_with_string(name: impl Into<String>) -> Self {
        Self {
            receivers: Arc::new(RwLock::new(Vec::new())),
            middlewares: Arc::new(RwLock::new(Vec::new())),
            context: SignalContext::new(),
            metrics: Arc::new(MetricsCollector::new()),
            name: name.into(),
        }
    }

    /// Get the current metrics for this signal
    pub fn metrics(&self) -> SignalMetrics {
        self.metrics.snapshot()
    }

    /// Reset metrics for this signal
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    /// Add middleware to this signal
    pub fn add_middleware<M>(&self, middleware: M)
    where
        M: SignalMiddleware<T> + 'static,
    {
        let mut middlewares = self.middlewares.write();
        middlewares.push(Arc::new(middleware));
    }

    /// Get the signal context
    pub fn context(&self) -> &SignalContext {
        &self.context
    }

    /// Connect a receiver function to this signal with full options
    ///
    /// # Arguments
    /// * `receiver` - The receiver function to connect
    /// * `sender_type_id` - Optional TypeId to filter by sender type
    /// * `dispatch_uid` - Optional unique identifier to prevent duplicate registration
    /// * `priority` - Execution priority (higher values execute first, default: 0)
    pub fn connect_with_options<F, Fut>(
        &self,
        receiver: F,
        sender_type_id: Option<TypeId>,
        dispatch_uid: Option<String>,
        priority: i32,
    ) where
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
    {
        self.connect_with_full_options::<F, Fut, fn(&T) -> bool>(
            receiver,
            sender_type_id,
            dispatch_uid,
            priority,
            None,
        );
    }

    /// Connect a receiver with all available options including predicate
    ///
    /// # Arguments
    /// * `receiver` - The receiver function to connect
    /// * `sender_type_id` - Optional TypeId to filter by sender type
    /// * `dispatch_uid` - Optional unique identifier to prevent duplicate registration
    /// * `priority` - Execution priority (higher values execute first, default: 0)
    /// * `predicate` - Optional condition that must be true for receiver to execute
    pub fn connect_with_full_options<F, Fut, P>(
        &self,
        receiver: F,
        sender_type_id: Option<TypeId>,
        dispatch_uid: Option<String>,
        priority: i32,
        predicate: Option<P>,
    ) where
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let boxed: ReceiverFn<T> = Arc::new(move |instance| Box::pin(receiver(instance)));
        let pred: Option<PredicateFn<T>> = predicate.map(|p| Arc::new(p) as PredicateFn<T>);
        let mut receivers = self.receivers.write();

        // Remove existing receiver with same dispatch_uid
        if let Some(ref uid) = dispatch_uid {
            receivers.retain(|r| r.dispatch_uid.as_ref() != Some(uid));
        }

        receivers.push(ReceiverInfo {
            receiver: boxed,
            sender_type_id,
            dispatch_uid,
            priority,
            predicate: pred,
        });

        // Sort by priority (descending - higher priority first)
        receivers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Connect a receiver function to this signal (simple version)
    pub fn connect<F, Fut>(&self, receiver: F)
    where
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
    {
        self.connect_with_options(receiver, None, None, 0);
    }

    /// Connect a receiver with priority
    pub fn connect_with_priority<F, Fut>(&self, receiver: F, priority: i32)
    where
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
    {
        self.connect_with_options(receiver, None, None, priority);
    }

    /// Chain this signal to another signal
    /// When this signal is sent, it will also trigger the chained signal
    ///
    /// # Example
    /// ```rust,ignore
    /// let signal_a = Signal::<User>::new("user_created");
    /// let signal_b = Signal::<User>::new("user_notified");
    ///
    /// signal_a.chain(&signal_b);
    ///
    // Now sending signal_a will also trigger signal_b
    /// signal_a.send(user).await?;
    /// ```
    pub fn chain(&self, next: &Signal<T>)
    where
        T: Clone,
    {
        let next_clone = next.clone();
        self.connect(move |instance| {
            let next = next_clone.clone();
            async move {
                let value = (*instance).clone();
                next.send(value).await
            }
        });
    }

    /// Chain this signal to another signal with transformation
    /// Allows transforming the instance before passing to the next signal
    ///
    /// # Example
    /// ```rust,ignore
    /// signal_a.chain_with(signal_b, |user| {
    ///     // Transform User to NotificationPayload
    ///     NotificationPayload::from(user)
    /// });
    /// ```
    pub fn chain_with<U, F>(&self, next: &Signal<U>, transform: F)
    where
        U: Send + Sync + 'static,
        F: Fn(Arc<T>) -> U + Send + Sync + 'static,
    {
        let next_clone = next.clone();
        let transform = Arc::new(transform);
        self.connect(move |instance| {
            let next = next_clone.clone();
            let transform = transform.clone();
            async move {
                let transformed = transform(instance);
                next.send(transformed).await
            }
        });
    }

    /// Merge multiple signals into one
    /// Returns a new signal that triggers when any of the source signals trigger
    ///
    /// # Example
    /// ```rust,ignore
    /// let merged = Signal::merge(vec![&signal_a, &signal_b, &signal_c]);
    ///
    /// merged.connect(|instance| async move {
    ///     println!("Any of the three signals was triggered!");
    ///     Ok(())
    /// });
    /// ```
    pub fn merge(signals: Vec<&Signal<T>>) -> Signal<T>
    where
        T: Clone,
    {
        let merged = Signal::new(SignalName::custom("merged_signal"));

        for signal in signals {
            let merged_clone = merged.clone();
            signal.connect(move |instance| {
                let merged = merged_clone.clone();
                async move {
                    let value = (*instance).clone();
                    merged.send(value).await
                }
            });
        }

        merged
    }

    /// Filter signal emissions based on a predicate
    /// Returns a new signal that only triggers when the predicate returns true
    ///
    /// # Example
    /// ```rust,ignore
    /// let admin_only = user_signal.filter(|user| user.is_admin);
    ///
    /// admin_only.connect(|admin_user| async move {
    ///     println!("Admin user action!");
    ///     Ok(())
    /// });
    /// ```
    pub fn filter<P>(&self, predicate: P) -> Signal<T>
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
        T: Clone,
    {
        let filtered = Signal::new_with_string(format!("{}_filtered", self.name));
        let predicate = Arc::new(predicate);
        let filtered_clone = filtered.clone();

        self.connect(move |instance| {
            let filtered = filtered_clone.clone();
            let predicate = predicate.clone();
            async move {
                if predicate(&instance) {
                    let value = (*instance).clone();
                    filtered.send(value).await
                } else {
                    Ok(())
                }
            }
        });

        filtered
    }

    /// Map signal emissions through a transformation function
    /// Returns a new signal with transformed values
    ///
    /// # Example
    /// ```rust,ignore
    /// let user_ids = user_signal.map(|user| user.id);
    ///
    /// user_ids.connect(|id| async move {
    ///     println!("User ID: {}", id);
    ///     Ok(())
    /// });
    /// ```
    pub fn map<U, F>(&self, transform: F) -> Signal<U>
    where
        U: Send + Sync + 'static,
        F: Fn(Arc<T>) -> U + Send + Sync + 'static,
    {
        let mapped = Signal::new_with_string(format!("{}_mapped", self.name));
        let transform = Arc::new(transform);
        let mapped_clone = mapped.clone();

        self.connect(move |instance| {
            let mapped = mapped_clone.clone();
            let transform = transform.clone();
            async move {
                let transformed = transform(instance);
                mapped.send(transformed).await
            }
        });

        mapped
    }

    /// Connect a receiver with a predicate condition
    ///
    /// The receiver will only execute if the predicate returns true for the instance
    pub fn connect_if<F, Fut, P>(&self, receiver: F, predicate: P)
    where
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SignalError>> + Send + 'static,
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.connect_with_full_options(receiver, None, None, 0, Some(predicate));
    }

    /// Disconnect a receiver by dispatch_uid
    pub fn disconnect(&self, dispatch_uid: &str) -> bool {
        let mut receivers = self.receivers.write();
        let original_len = receivers.len();
        receivers.retain(|r| r.dispatch_uid.as_ref().map(|s| s.as_str()) != Some(dispatch_uid));
        receivers.len() < original_len
    }

    /// Send signal to all connected receivers
    ///
    /// # Arguments
    /// * `instance` - The instance to send
    /// * `sender_type_id` - Optional TypeId of the sender for filtering
    pub async fn send_with_sender(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Result<(), SignalError> {
        // Record send event
        self.metrics.record_send();

        let instance = Arc::new(instance);
        let receivers = self.receivers.read().clone();
        let middlewares = self.middlewares.read().clone();

        // Execute before_send middleware hooks
        for middleware in &middlewares {
            let should_continue = middleware.before_send(&instance).await?;
            if !should_continue {
                return Ok(()); // Middleware stopped signal propagation
            }
        }

        let mut results = Vec::new();

        for receiver_info in receivers {
            // Check sender type match
            if let Some(expected_type_id) = receiver_info.sender_type_id {
                if let Some(actual_type_id) = sender_type_id {
                    if expected_type_id != actual_type_id {
                        continue; // Type mismatch, skip this receiver
                    }
                } else {
                    continue; // Receiver expects a specific sender, but None was provided
                }
            }

            // Check predicate condition
            if let Some(ref predicate) = receiver_info.predicate {
                if !predicate(&instance) {
                    continue; // Predicate failed, skip this receiver
                }
            }

            // Execute before_receiver middleware hooks
            let dispatch_uid_ref = receiver_info.dispatch_uid.as_deref();
            let mut should_execute = true;
            for middleware in &middlewares {
                let can_execute = middleware
                    .before_receiver(&instance, dispatch_uid_ref)
                    .await?;
                if !can_execute {
                    should_execute = false;
                    break;
                }
            }

            if !should_execute {
                continue; // Middleware skipped this receiver
            }

            // Execute receiver and measure time
            let start = Instant::now();
            let result = (receiver_info.receiver)(Arc::clone(&instance)).await;
            let duration = start.elapsed();

            // Record metrics
            self.metrics
                .record_receiver_execution(duration, result.is_ok());

            // Execute after_receiver middleware hooks
            for middleware in &middlewares {
                middleware
                    .after_receiver(&instance, dispatch_uid_ref, &result)
                    .await?;
            }

            // Stop on first error (not robust mode)
            result?;
            results.push(Ok(()));
        }

        // Execute after_send middleware hooks
        for middleware in &middlewares {
            middleware.after_send(&instance, &results).await?;
        }

        Ok(())
    }

    /// Send signal to all connected receivers (simple version)
    pub async fn send(&self, instance: T) -> Result<(), SignalError> {
        self.send_with_sender(instance, None).await
    }

    /// Send signal robustly, catching errors
    pub async fn send_robust(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Vec<Result<(), SignalError>> {
        // Record send event
        self.metrics.record_send();

        let instance = Arc::new(instance);
        let receivers = self.receivers.read().clone();
        let middlewares = self.middlewares.read().clone();
        let mut results = Vec::new();

        // Execute before_send middleware hooks (ignore errors in robust mode)
        for middleware in &middlewares {
            if let Ok(should_continue) = middleware.before_send(&instance).await {
                if !should_continue {
                    return results; // Middleware stopped signal propagation
                }
            }
        }

        for receiver_info in receivers {
            // Check sender type match
            if let Some(expected_type_id) = receiver_info.sender_type_id {
                if let Some(actual_type_id) = sender_type_id {
                    if expected_type_id != actual_type_id {
                        continue; // Type mismatch, skip this receiver
                    }
                } else {
                    continue; // Receiver expects a specific sender, but None was provided
                }
            }

            // Check predicate condition
            if let Some(ref predicate) = receiver_info.predicate {
                if !predicate(&instance) {
                    continue; // Predicate failed, skip this receiver
                }
            }

            // Execute before_receiver middleware hooks
            let dispatch_uid_ref = receiver_info.dispatch_uid.as_deref();
            let mut should_execute = true;
            for middleware in &middlewares {
                if let Ok(can_execute) = middleware
                    .before_receiver(&instance, dispatch_uid_ref)
                    .await
                {
                    if !can_execute {
                        should_execute = false;
                        break;
                    }
                }
            }

            if !should_execute {
                continue; // Middleware skipped this receiver
            }

            // Execute receiver and measure time
            let start = Instant::now();
            let result = (receiver_info.receiver)(Arc::clone(&instance)).await;
            let duration = start.elapsed();

            // Record metrics
            self.metrics
                .record_receiver_execution(duration, result.is_ok());

            // Execute after_receiver middleware hooks (ignore errors)
            for middleware in &middlewares {
                let _ = middleware
                    .after_receiver(&instance, dispatch_uid_ref, &result)
                    .await;
            }

            results.push(result);
        }

        // Execute after_send middleware hooks (ignore errors)
        for middleware in &middlewares {
            let _ = middleware.after_send(&instance, &results).await;
        }

        results
    }

    /// Send signal asynchronously (fire and forget)
    pub fn send_async(&self, instance: T) {
        let instance = Arc::new(instance);
        let receivers = self.receivers.read().clone();

        tokio::spawn(async move {
            for receiver_info in receivers {
                let _ = (receiver_info.receiver)(Arc::clone(&instance)).await;
            }
        });
    }

    /// Get number of connected receivers
    pub fn receiver_count(&self) -> usize {
        self.receivers.read().len()
    }

    /// Clear all receivers
    pub fn disconnect_all(&self) {
        self.receivers.write().clear();
    }
}

impl<T: Send + Sync + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            receivers: Arc::clone(&self.receivers),
            middlewares: Arc::clone(&self.middlewares),
            context: self.context.clone(),
            metrics: Arc::clone(&self.metrics),
            name: self.name.clone(),
        }
    }
}

impl<T: Send + Sync + 'static> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("name", &self.name)
            .field("receiver_count", &self.receiver_count())
            .finish()
    }
}

// Implement SignalDispatcher trait for Signal
impl<T: Send + Sync + 'static> SignalDispatcher<T> for Signal<T> {
    fn receiver_count(&self) -> usize {
        Signal::receiver_count(self)
    }

    fn disconnect_all(&self) {
        Signal::disconnect_all(self)
    }

    fn disconnect(&self, dispatch_uid: &str) -> bool {
        Signal::disconnect(self, dispatch_uid)
    }
}

// Implement AsyncSignalDispatcher trait for Signal
#[async_trait::async_trait]
impl<T: Send + Sync + 'static> AsyncSignalDispatcher<T> for Signal<T> {
    async fn send(&self, instance: T) -> Result<(), SignalError> {
        Signal::send(self, instance).await
    }

    async fn send_with_sender(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Result<(), SignalError> {
        Signal::send_with_sender(self, instance, sender_type_id).await
    }

    async fn send_robust(
        &self,
        instance: T,
        sender_type_id: Option<TypeId>,
    ) -> Vec<Result<(), SignalError>> {
        Signal::send_robust(self, instance, sender_type_id).await
    }
}

/// Global signal registry
pub struct SignalRegistry {
    signals: RwLock<HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>>,
}

impl SignalRegistry {
    fn new() -> Self {
        Self {
            signals: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a signal for a specific type and name
    pub fn get_or_create<T: Send + Sync + 'static>(&self, name: SignalName) -> Signal<T> {
        let type_id = TypeId::of::<T>();
        let key = (type_id, name.as_str().to_string());

        // Try to get existing signal
        {
            let signals = self.signals.read();
            if let Some(signal_any) = signals.get(&key) {
                if let Some(signal) = signal_any.downcast_ref::<Signal<T>>() {
                    return signal.clone();
                }
            }
        }

        // Create new signal
        let signal = Signal::new(name);
        self.signals.write().insert(key, Box::new(signal.clone()));
        signal
    }

    /// Get or create a signal with a string name (for backward compatibility)
    #[doc(hidden)]
    pub fn get_or_create_with_string<T: Send + Sync + 'static>(
        &self,
        name: impl Into<String>,
    ) -> Signal<T> {
        let name_str = name.into();
        let type_id = TypeId::of::<T>();
        let key = (type_id, name_str.clone());

        // Try to get existing signal
        {
            let signals = self.signals.read();
            if let Some(signal_any) = signals.get(&key) {
                if let Some(signal) = signal_any.downcast_ref::<Signal<T>>() {
                    return signal.clone();
                }
            }
        }

        // Create new signal (need to leak string for SignalName)
        let leaked: &'static str = Box::leak(name_str.clone().into_boxed_str());
        let signal = Signal::new(SignalName::custom(leaked));
        self.signals.write().insert(key, Box::new(signal.clone()));
        signal
    }
}

// Global registry instance
static GLOBAL_REGISTRY: once_cell::sync::Lazy<SignalRegistry> =
    once_cell::sync::Lazy::new(|| SignalRegistry::new());

/// Get a signal from the global registry with type-safe name
pub fn get_signal<T: Send + Sync + 'static>(name: SignalName) -> Signal<T> {
    GLOBAL_REGISTRY.get_or_create(name)
}

/// Get a signal from the global registry with string name (for backward compatibility)
#[doc(hidden)]
pub fn get_signal_with_string<T: Send + Sync + 'static>(name: impl Into<String>) -> Signal<T> {
    GLOBAL_REGISTRY.get_or_create_with_string(name)
}

/// Pre-save signal - sent before a model instance is saved
pub fn pre_save<T: Send + Sync + 'static>() -> Signal<T> {
    get_signal::<T>(SignalName::PRE_SAVE)
}

/// Post-save signal - sent after a model instance is saved
pub fn post_save<T: Send + Sync + 'static>() -> Signal<T> {
    get_signal::<T>(SignalName::POST_SAVE)
}

/// Pre-delete signal - sent before a model instance is deleted
pub fn pre_delete<T: Send + Sync + 'static>() -> Signal<T> {
    get_signal::<T>(SignalName::PRE_DELETE)
}

/// Post-delete signal - sent after a model instance is deleted
pub fn post_delete<T: Send + Sync + 'static>() -> Signal<T> {
    get_signal::<T>(SignalName::POST_DELETE)
}

/// M2M changed signal - sent when many-to-many relationships change
#[derive(Debug, Clone)]
pub struct M2MChangeEvent<T, R> {
    pub instance: T,
    pub action: M2MAction,
    pub related: Vec<R>,
    pub reverse: bool,
    pub model_name: String,
}

impl<T, R> M2MChangeEvent<T, R> {
    pub fn new(instance: T, action: M2MAction, related: Vec<R>) -> Self {
        Self {
            instance,
            action,
            related,
            reverse: false,
            model_name: String::new(),
        }
    }

    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn with_model_name(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = model_name.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M2MAction {
    PreAdd,
    PostAdd,
    PreRemove,
    PostRemove,
    PreClear,
    PostClear,
}

impl fmt::Display for M2MAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            M2MAction::PreAdd => write!(f, "pre_add"),
            M2MAction::PostAdd => write!(f, "post_add"),
            M2MAction::PreRemove => write!(f, "pre_remove"),
            M2MAction::PostRemove => write!(f, "post_remove"),
            M2MAction::PreClear => write!(f, "pre_clear"),
            M2MAction::PostClear => write!(f, "post_clear"),
        }
    }
}

pub fn m2m_changed<T: Send + Sync + 'static, R: Send + Sync + 'static>(
) -> Signal<M2MChangeEvent<T, R>> {
    get_signal::<M2MChangeEvent<T, R>>(SignalName::M2M_CHANGED)
}

/// Pre-init signal - sent at the beginning of a model's __init__ method
#[derive(Debug, Clone)]
pub struct PreInitEvent<T> {
    pub model_type: String,
    pub args: Vec<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> PreInitEvent<T> {
    pub fn new(model_type: impl Into<String>) -> Self {
        Self {
            model_type: model_type.into(),
            args: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }
}

pub fn pre_init<T: Send + Sync + 'static>() -> Signal<PreInitEvent<T>> {
    get_signal::<PreInitEvent<T>>(SignalName::PRE_INIT)
}

/// Post-init signal - sent at the end of a model's __init__ method
#[derive(Debug, Clone)]
pub struct PostInitEvent<T> {
    pub instance: T,
}

impl<T> PostInitEvent<T> {
    pub fn new(instance: T) -> Self {
        Self { instance }
    }
}

pub fn post_init<T: Send + Sync + 'static>() -> Signal<PostInitEvent<T>> {
    get_signal::<PostInitEvent<T>>(SignalName::POST_INIT)
}

/// Pre-migrate signal - sent before running migrations
#[derive(Debug, Clone)]
pub struct MigrationEvent {
    pub app_name: String,
    pub migration_name: String,
    pub plan: Vec<String>,
}

impl MigrationEvent {
    pub fn new(app_name: impl Into<String>, migration_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            migration_name: migration_name.into(),
            plan: Vec::new(),
        }
    }

    pub fn with_plan(mut self, plan: Vec<String>) -> Self {
        self.plan = plan;
        self
    }
}

pub fn pre_migrate() -> Signal<MigrationEvent> {
    get_signal::<MigrationEvent>(SignalName::PRE_MIGRATE)
}

/// Post-migrate signal - sent after running migrations
pub fn post_migrate() -> Signal<MigrationEvent> {
    get_signal::<MigrationEvent>(SignalName::POST_MIGRATE)
}

/// Class prepared signal - sent when a model class is prepared
#[derive(Debug, Clone)]
pub struct ClassPreparedEvent {
    pub model_name: String,
    pub app_label: String,
}

impl ClassPreparedEvent {
    pub fn new(model_name: impl Into<String>, app_label: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            app_label: app_label.into(),
        }
    }
}

pub fn class_prepared() -> Signal<ClassPreparedEvent> {
    get_signal::<ClassPreparedEvent>(SignalName::CLASS_PREPARED)
}

/// Request started signal - sent when an HTTP request starts
#[derive(Debug, Clone)]
pub struct RequestStartedEvent {
    pub environ: HashMap<String, String>,
}

impl RequestStartedEvent {
    pub fn new() -> Self {
        Self {
            environ: HashMap::new(),
        }
    }

    pub fn with_environ(mut self, environ: HashMap<String, String>) -> Self {
        self.environ = environ;
        self
    }
}

impl Default for RequestStartedEvent {
    fn default() -> Self {
        Self::new()
    }
}

pub fn request_started() -> Signal<RequestStartedEvent> {
    get_signal::<RequestStartedEvent>(SignalName::REQUEST_STARTED)
}

/// Request finished signal - sent when an HTTP request finishes
#[derive(Debug, Clone)]
pub struct RequestFinishedEvent {
    pub environ: HashMap<String, String>,
}

impl RequestFinishedEvent {
    pub fn new() -> Self {
        Self {
            environ: HashMap::new(),
        }
    }

    pub fn with_environ(mut self, environ: HashMap<String, String>) -> Self {
        self.environ = environ;
        self
    }
}

impl Default for RequestFinishedEvent {
    fn default() -> Self {
        Self::new()
    }
}

pub fn request_finished() -> Signal<RequestFinishedEvent> {
    get_signal::<RequestFinishedEvent>(SignalName::REQUEST_FINISHED)
}

/// Got request exception signal - sent when an exception occurs during request handling
#[derive(Debug, Clone)]
pub struct GotRequestExceptionEvent {
    pub error_message: String,
    pub request_info: HashMap<String, String>,
}

impl GotRequestExceptionEvent {
    pub fn new(error_message: impl Into<String>) -> Self {
        Self {
            error_message: error_message.into(),
            request_info: HashMap::new(),
        }
    }

    pub fn with_request_info(mut self, request_info: HashMap<String, String>) -> Self {
        self.request_info = request_info;
        self
    }
}

pub fn got_request_exception() -> Signal<GotRequestExceptionEvent> {
    get_signal::<GotRequestExceptionEvent>(SignalName::GOT_REQUEST_EXCEPTION)
}

/// Setting changed signal - sent when a setting is changed
#[derive(Debug, Clone)]
pub struct SettingChangedEvent {
    pub setting_name: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

impl SettingChangedEvent {
    pub fn new(
        setting_name: impl Into<String>,
        old_value: Option<String>,
        new_value: impl Into<String>,
    ) -> Self {
        Self {
            setting_name: setting_name.into(),
            old_value,
            new_value: new_value.into(),
        }
    }
}

pub fn setting_changed() -> Signal<SettingChangedEvent> {
    get_signal::<SettingChangedEvent>(SignalName::SETTING_CHANGED)
}

/// Helper macro for connecting receivers with a simpler syntax
///
/// # Example
///
/// ```ignore
/// use reinhardt_signals::{connect_receiver, post_save};
///
/// connect_receiver!(post_save::<User>(), on_user_saved);
///
/// async fn on_user_saved(instance: Arc<User>) -> Result<(), SignalError> {
///     println!("User saved: {:?}", instance);
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! connect_receiver {
    ($signal:expr, $receiver:expr) => {
        $signal.connect($receiver)
    };
    ($signal:expr, $receiver:expr, priority = $priority:expr) => {
        $signal.connect_with_priority($receiver, $priority)
    };
    ($signal:expr, $receiver:expr, dispatch_uid = $uid:expr) => {
        $signal.connect_with_options($receiver, None, Some($uid.to_string()), 0)
    };
    ($signal:expr, $receiver:expr, sender = $sender:ty) => {
        $signal.connect_with_options($receiver, Some(std::any::TypeId::of::<$sender>()), None, 0)
    };
    ($signal:expr, $receiver:expr, priority = $priority:expr, dispatch_uid = $uid:expr) => {
        $signal.connect_with_options($receiver, None, Some($uid.to_string()), $priority)
    };
    ($signal:expr, $receiver:expr, sender = $sender:ty, dispatch_uid = $uid:expr) => {
        $signal.connect_with_options(
            $receiver,
            Some(std::any::TypeId::of::<$sender>()),
            Some($uid.to_string()),
            0,
        )
    };
    ($signal:expr, $receiver:expr, sender = $sender:ty, priority = $priority:expr) => {
        $signal.connect_with_options(
            $receiver,
            Some(std::any::TypeId::of::<$sender>()),
            None,
            $priority,
        )
    };
    ($signal:expr, $receiver:expr, sender = $sender:ty, dispatch_uid = $uid:expr, priority = $priority:expr) => {
        $signal.connect_with_options(
            $receiver,
            Some(std::any::TypeId::of::<$sender>()),
            Some($uid.to_string()),
            $priority,
        )
    };
}

/// Database lifecycle event helpers
///
/// These helpers provide convenient database lifecycle events similar to SQLAlchemy,
/// integrated with the Django-style signal system.
pub mod db_events {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// Generic database event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DbEvent {
        pub event_type: String,
        pub table: String,
        pub id: Option<String>,
        pub data: HashMap<String, String>,
    }

    impl DbEvent {
        pub fn new(event_type: impl Into<String>, table: impl Into<String>) -> Self {
            Self {
                event_type: event_type.into(),
                table: table.into(),
                id: None,
                data: HashMap::new(),
            }
        }

        pub fn with_id(mut self, id: impl Into<String>) -> Self {
            self.id = Some(id.into());
            self
        }

        pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
            self.data.insert(key.into(), value.into());
            self
        }
    }

    /// Before insert signal
    pub fn before_insert() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_BEFORE_INSERT)
    }

    /// After insert signal
    pub fn after_insert() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_AFTER_INSERT)
    }

    /// Before update signal
    pub fn before_update() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_BEFORE_UPDATE)
    }

    /// After update signal
    pub fn after_update() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_AFTER_UPDATE)
    }

    /// Before delete signal
    pub fn before_delete() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_BEFORE_DELETE)
    }

    /// After delete signal
    pub fn after_delete() -> Signal<DbEvent> {
        get_signal::<DbEvent>(SignalName::DB_AFTER_DELETE)
    }
}

/// Django-style synchronous signal dispatcher
///
/// This module provides a synchronous signal system compatible with Django's dispatch pattern.
pub mod dispatch {
    use parking_lot::RwLock;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::{Arc, Weak};

    /// Receiver function type for synchronous signals
    pub type SyncReceiverFn = Arc<
        dyn Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String
            + Send
            + Sync,
    >;

    /// Synchronous signal that mimics Django's Signal class
    #[derive(Clone)]
    pub struct SyncSignal {
        receivers: Arc<RwLock<Vec<SignalReceiver>>>,
        #[allow(dead_code)]
        use_caching: bool,
    }

    struct SignalReceiver {
        receiver: Weak<
            dyn Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String
                + Send
                + Sync,
        >,
        sender_type_id: Option<std::any::TypeId>,
        dispatch_uid: Option<String>,
        // Keep a strong reference to prevent premature deallocation (when caller transfers ownership)
        _strong_ref: Option<SyncReceiverFn>,
    }

    impl SyncSignal {
        /// Create a new synchronous signal
        pub fn new() -> Self {
            Self {
                receivers: Arc::new(RwLock::new(Vec::new())),
                use_caching: false,
            }
        }

        /// Create a new synchronous signal with caching
        pub fn new_with_caching() -> Self {
            Self {
                receivers: Arc::new(RwLock::new(Vec::new())),
                use_caching: true,
            }
        }

        /// Connect a receiver to this signal
        pub fn connect<F>(
            &self,
            receiver: Arc<F>,
            sender_type_id: Option<std::any::TypeId>,
            dispatch_uid: Option<String>,
        ) -> Result<(), String>
        where
            F: Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String
                + Send
                + Sync
                + 'static,
        {
            // Check if caller has other references before converting
            let should_store_strong = Arc::strong_count(&receiver) == 1;

            // Store the Arc as a trait object
            let receiver_arc: SyncReceiverFn = receiver;
            let weak_receiver = Arc::downgrade(&receiver_arc);
            let mut receivers = self.receivers.write();

            // Remove existing receiver with same dispatch_uid
            if let Some(ref uid) = dispatch_uid {
                receivers.retain(|r| r.dispatch_uid.as_ref() != Some(uid));
            }

            // Prevent duplicate registrations
            let receiver_ptr = weak_receiver.as_ptr();
            receivers.retain(|r| !std::ptr::addr_eq(r.receiver.as_ptr(), receiver_ptr));

            receivers.push(SignalReceiver {
                receiver: weak_receiver,
                sender_type_id,
                dispatch_uid,
                // Only store strong ref if caller has no other references (ownership transfer)
                _strong_ref: if should_store_strong {
                    Some(receiver_arc)
                } else {
                    None
                },
            });

            Ok(())
        }

        /// Disconnect a receiver by dispatch_uid
        /// If dispatch_uid is None, disconnects all receivers
        pub fn disconnect(&self, dispatch_uid: Option<&str>) -> bool {
            let mut receivers = self.receivers.write();
            let original_len = receivers.len();

            if let Some(uid) = dispatch_uid {
                receivers.retain(|r| r.dispatch_uid.as_ref().map(|s| s.as_str()) != Some(uid));
            } else {
                // If no dispatch_uid provided, clear all receivers
                receivers.clear();
            }

            receivers.len() < original_len
        }

        /// Send signal to all connected receivers
        pub fn send(
            &self,
            sender: Option<Arc<dyn Any + Send + Sync>>,
            kwargs: &HashMap<String, String>,
        ) -> Vec<(String, String)> {
            self.clear_dead_receivers();
            let receivers = self.receivers.read();
            let mut results = Vec::new();

            for receiver_data in receivers.iter() {
                // Check sender type match
                if let Some(expected_type_id) = receiver_data.sender_type_id {
                    if let Some(ref actual_sender) = sender {
                        // Must explicitly dereference Arc to get the underlying TypeId
                        if (**actual_sender).type_id() != expected_type_id {
                            continue; // Type mismatch
                        }
                    } else {
                        continue; // Receiver expects a specific sender, but None was provided
                    }
                }

                if let Some(receiver) = receiver_data.receiver.upgrade() {
                    let result = receiver(sender.clone(), kwargs);
                    results.push(("receiver".to_string(), result));
                }
            }

            results
        }

        /// Send signal robustly (catching panics)
        pub fn send_robust(
            &self,
            sender: Option<Arc<dyn Any + Send + Sync>>,
            kwargs: &HashMap<String, String>,
        ) -> Vec<(String, Result<String, String>)> {
            self.clear_dead_receivers();
            let receivers = self.receivers.read();
            let mut results = Vec::new();

            for receiver_data in receivers.iter() {
                if let Some(receiver) = receiver_data.receiver.upgrade() {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        receiver(sender.clone(), kwargs)
                    }));

                    match result {
                        Ok(val) => results.push(("receiver".to_string(), Ok(val))),
                        Err(_) => results.push(("receiver".to_string(), Err("panic".to_string()))),
                    }
                }
            }

            results
        }

        /// Check if signal has any listeners
        pub fn has_listeners(&self) -> bool {
            self.clear_dead_receivers();
            let receivers = self.receivers.read();
            !receivers.is_empty()
        }

        /// Get receiver count
        pub fn receivers_count(&self) -> usize {
            self.receivers.read().len()
        }

        /// Clear dead (garbage collected) receivers
        pub fn clear_dead_receivers(&self) {
            let mut receivers = self.receivers.write();
            receivers.retain(|r| r.receiver.strong_count() > 0);
        }
    }

    impl Default for SyncSignal {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct TestModel {
        id: i32,
        name: String,
    }

    #[tokio::test]
    async fn test_signal_connect_and_send() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        signal.connect(move |_instance: Arc<TestModel>| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_signals_multiple_receivers() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        // Connect multiple receivers
        for _ in 0..3 {
            let counter_clone = Arc::clone(&counter);
            signal.connect(move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            });
        }

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_signals_pre_post_save() {
        let pre_counter = Arc::new(AtomicUsize::new(0));
        let post_counter = Arc::new(AtomicUsize::new(0));

        // Connect to pre_save
        let pre_clone = Arc::clone(&pre_counter);
        pre_save::<TestModel>().connect(move |_instance| {
            let counter = Arc::clone(&pre_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        // Connect to post_save
        let post_clone = Arc::clone(&post_counter);
        post_save::<TestModel>().connect(move |_instance| {
            let counter = Arc::clone(&post_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        // Simulate save operation
        pre_save::<TestModel>().send(model.clone()).await.unwrap();
        // ... actual save would happen here ...
        post_save::<TestModel>().send(model).await.unwrap();

        assert_eq!(pre_counter.load(Ordering::SeqCst), 1);
        assert_eq!(post_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_signals_global_registry() {
        let signal1 = get_signal::<TestModel>(SignalName::custom("custom_signal"));
        let signal2 = get_signal::<TestModel>(SignalName::custom("custom_signal"));

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        // Connect to signal1
        signal1.connect(move |_instance| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        // Send through signal2 (should be the same signal)
        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        signal2.send(model).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dispatch_uid() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            None,
            Some("unique_id".to_string()),
            0,
        );

        // Try to connect again with same dispatch_uid (should replace)
        let counter_clone2 = Arc::clone(&counter);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone2);
                async move {
                    counter.fetch_add(10, Ordering::SeqCst);
                    Ok(())
                }
            },
            None,
            Some("unique_id".to_string()),
            0,
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        // Should only execute the second receiver (10), not both
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_sender_filtering() {
        struct SenderA;
        struct SenderB;

        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        // Connect receiver that only listens to SenderA
        let counter_clone = Arc::clone(&counter);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            Some(TypeId::of::<SenderA>()),
            None,
            0,
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        // Send from SenderA - should trigger receiver
        signal
            .send_with_sender(model.clone(), Some(TypeId::of::<SenderA>()))
            .await
            .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Send from SenderB - should NOT trigger receiver
        signal
            .send_with_sender(model, Some(TypeId::of::<SenderB>()))
            .await
            .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_send_robust() {
        let signal = Signal::new(SignalName::custom("test"));

        // Connect a failing receiver
        signal.connect(|_instance: Arc<TestModel>| async move {
            Err(SignalError::new("First receiver failed"))
        });

        // Connect a successful receiver
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        let results = signal.send_robust(model, None).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_err());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_disconnect() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            None,
            Some("test_receiver".to_string()),
            0,
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        // Send signal - should trigger receiver
        signal.send(model.clone()).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Disconnect the receiver
        assert!(signal.disconnect("test_receiver"));

        // Send again - should NOT trigger receiver
        signal.send(model).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_connect_receiver_macro() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);

        // Use the connect_receiver macro
        connect_receiver!(signal, move |_instance: Arc<TestModel>| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_connect_receiver_macro_with_dispatch_uid() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);

        // Connect with dispatch_uid using macro
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            dispatch_uid = "unique_receiver"
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Should be able to disconnect
        assert!(signal.disconnect("unique_receiver"));
    }

    #[tokio::test]
    async fn test_priority_execution_order() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_order = Arc::new(RwLock::new(Vec::new()));

        // Connect receivers with different priorities
        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_priority(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push(1);
                    Ok(())
                }
            },
            10, // High priority
        );

        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_priority(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push(2);
                    Ok(())
                }
            },
            5, // Medium priority
        );

        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_priority(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push(3);
                    Ok(())
                }
            },
            1, // Low priority
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        // Verify execution order: high to low priority (10 -> 5 -> 1)
        let order = execution_order.read();
        assert_eq!(*order, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_priority_with_macro() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_order = Arc::new(RwLock::new(Vec::new()));

        // Test priority parameter in macro
        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("high");
                    Ok(())
                }
            },
            priority = 100
        );

        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("low");
                    Ok(())
                }
            },
            priority = -10
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        let order = execution_order.read();
        assert_eq!(*order, vec!["high", "low"]);
    }

    #[tokio::test]
    async fn test_priority_with_dispatch_uid() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_order = Arc::new(RwLock::new(Vec::new()));

        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("first");
                    Ok(())
                }
            },
            priority = 50,
            dispatch_uid = "test_handler"
        );

        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("second");
                    Ok(())
                }
            },
            priority = 100,
            dispatch_uid = "test_handler"
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        // Should only have "second" because dispatch_uid replaces
        let order = execution_order.read();
        assert_eq!(*order, vec!["second"]);
    }

    #[tokio::test]
    async fn test_priority_with_sender_filtering() {
        struct SenderA;
        #[allow(dead_code)]
        struct SenderB;

        let signal = Signal::new(SignalName::custom("test"));
        let execution_order = Arc::new(RwLock::new(Vec::new()));

        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("high_a");
                    Ok(())
                }
            },
            sender = SenderA,
            priority = 100
        );

        let order_clone = Arc::clone(&execution_order);
        connect_receiver!(
            signal,
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("low_a");
                    Ok(())
                }
            },
            sender = SenderA,
            priority = 10
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        // Send with SenderA
        signal
            .send_with_sender(model.clone(), Some(TypeId::of::<SenderA>()))
            .await
            .unwrap();

        let order = execution_order.read();
        assert_eq!(*order, vec!["high_a", "low_a"]);
    }

    #[tokio::test]
    async fn test_conditional_receiver() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        // Only execute for models with id > 10
        signal.connect_if(
            move |_instance: Arc<TestModel>| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            |model| model.id > 10,
        );

        // Send model with id=5 - should NOT trigger
        let model1 = TestModel {
            id: 5,
            name: "Test1".to_string(),
        };
        signal.send(model1).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        // Send model with id=15 - SHOULD trigger
        let model2 = TestModel {
            id: 15,
            name: "Test2".to_string(),
        };
        signal.send(model2).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_multiple_predicates() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_log = Arc::new(RwLock::new(Vec::new()));

        // Receiver 1: Only for even IDs
        let log_clone = Arc::clone(&execution_log);
        signal.connect_if(
            move |_instance: Arc<TestModel>| {
                let log = Arc::clone(&log_clone);
                async move {
                    log.write().push("even");
                    Ok(())
                }
            },
            |model| model.id % 2 == 0,
        );

        // Receiver 2: Only for IDs > 5
        let log_clone = Arc::clone(&execution_log);
        signal.connect_if(
            move |_instance: Arc<TestModel>| {
                let log = Arc::clone(&log_clone);
                async move {
                    log.write().push("greater_than_5");
                    Ok(())
                }
            },
            |model| model.id > 5,
        );

        // Receiver 3: Only for names starting with "A"
        let log_clone = Arc::clone(&execution_log);
        signal.connect_if(
            move |_instance: Arc<TestModel>| {
                let log = Arc::clone(&log_clone);
                async move {
                    log.write().push("starts_with_a");
                    Ok(())
                }
            },
            |model| model.name.starts_with('A'),
        );

        // Test 1: id=4, name="Test" -> only "even"
        let model1 = TestModel {
            id: 4,
            name: "Test".to_string(),
        };
        signal.send(model1).await.unwrap();
        {
            let log = execution_log.read();
            assert_eq!(*log, vec!["even"]);
        }
        execution_log.write().clear();

        // Test 2: id=7, name="Alice" -> "greater_than_5" and "starts_with_a"
        let model2 = TestModel {
            id: 7,
            name: "Alice".to_string(),
        };
        signal.send(model2).await.unwrap();
        {
            let log = execution_log.read();
            assert_eq!(*log, vec!["greater_than_5", "starts_with_a"]);
        }
        execution_log.write().clear();

        // Test 3: id=8, name="Bob" -> "even" and "greater_than_5"
        let model3 = TestModel {
            id: 8,
            name: "Bob".to_string(),
        };
        signal.send(model3).await.unwrap();
        {
            let log = execution_log.read();
            assert_eq!(*log, vec!["even", "greater_than_5"]);
        }
    }

    #[tokio::test]
    async fn test_predicate_with_send_robust() {
        let signal = Signal::new(SignalName::custom("test"));

        // Receiver that only executes for id > 10, but fails
        signal.connect_if(
            |_instance: Arc<TestModel>| async move { Err(SignalError::new("Intentional error")) },
            |model| model.id > 10,
        );

        // Receiver that always executes successfully
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Test with id=5 - first receiver skipped due to predicate
        let model1 = TestModel {
            id: 5,
            name: "Test".to_string(),
        };
        let results1 = signal.send_robust(model1, None).await;
        assert_eq!(results1.len(), 1); // Only second receiver executed
        assert!(results1[0].is_ok());

        // Test with id=15 - first receiver executes and fails
        let model2 = TestModel {
            id: 15,
            name: "Test".to_string(),
        };
        let results2 = signal.send_robust(model2, None).await;
        assert_eq!(results2.len(), 2); // Both receivers executed
        assert!(results2[0].is_err());
        assert!(results2[1].is_ok());
    }

    #[tokio::test]
    async fn test_predicate_with_priority() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_order = Arc::new(RwLock::new(Vec::new()));

        // High priority, but conditional (id > 10)
        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_full_options(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("high_conditional");
                    Ok(())
                }
            },
            None,
            None,
            100,
            Some(|model: &TestModel| model.id > 10),
        );

        // Medium priority, always executes
        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_priority(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("medium_always");
                    Ok(())
                }
            },
            50,
        );

        // Low priority, conditional (id <= 10)
        let order_clone = Arc::clone(&execution_order);
        signal.connect_with_full_options(
            move |_instance: Arc<TestModel>| {
                let order = Arc::clone(&order_clone);
                async move {
                    order.write().push("low_conditional");
                    Ok(())
                }
            },
            None,
            None,
            10,
            Some(|model: &TestModel| model.id <= 10),
        );

        // Test with id=5 - high priority skipped, medium and low execute
        let model1 = TestModel {
            id: 5,
            name: "Test".to_string(),
        };
        signal.send(model1).await.unwrap();
        {
            let order = execution_order.read();
            assert_eq!(*order, vec!["medium_always", "low_conditional"]);
        }
        execution_order.write().clear();

        // Test with id=15 - high priority and medium execute, low skipped
        let model2 = TestModel {
            id: 15,
            name: "Test".to_string(),
        };
        signal.send(model2).await.unwrap();
        {
            let order = execution_order.read();
            assert_eq!(*order, vec!["high_conditional", "medium_always"]);
        }
    }

    // Middleware tests
    struct LoggingMiddleware {
        log: Arc<RwLock<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl SignalMiddleware<TestModel> for LoggingMiddleware {
        async fn before_send(&self, instance: &TestModel) -> Result<bool, SignalError> {
            self.log
                .write()
                .push(format!("before_send: {}", instance.id));
            Ok(true)
        }

        async fn after_send(
            &self,
            instance: &TestModel,
            _results: &[Result<(), SignalError>],
        ) -> Result<(), SignalError> {
            self.log
                .write()
                .push(format!("after_send: {}", instance.id));
            Ok(())
        }

        async fn before_receiver(
            &self,
            instance: &TestModel,
            dispatch_uid: Option<&str>,
        ) -> Result<bool, SignalError> {
            self.log.write().push(format!(
                "before_receiver: {} ({})",
                instance.id,
                dispatch_uid.unwrap_or("none")
            ));
            Ok(true)
        }

        async fn after_receiver(
            &self,
            instance: &TestModel,
            dispatch_uid: Option<&str>,
            result: &Result<(), SignalError>,
        ) -> Result<(), SignalError> {
            self.log.write().push(format!(
                "after_receiver: {} ({}) - {}",
                instance.id,
                dispatch_uid.unwrap_or("none"),
                if result.is_ok() { "ok" } else { "err" }
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_hooks() {
        let signal = Signal::new(SignalName::custom("test"));
        let log = Arc::new(RwLock::new(Vec::new()));

        let middleware = LoggingMiddleware {
            log: Arc::clone(&log),
        };
        signal.add_middleware(middleware);

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 42,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        let log_entries = log.read();
        assert_eq!(log_entries[0], "before_send: 42");
        assert_eq!(log_entries[1], "before_receiver: 42 (none)");
        assert_eq!(log_entries[2], "after_receiver: 42 (none) - ok");
        assert_eq!(log_entries[3], "after_send: 42");
    }

    struct BlockingMiddleware {
        should_block: bool,
    }

    #[async_trait::async_trait]
    impl SignalMiddleware<TestModel> for BlockingMiddleware {
        async fn before_send(&self, _instance: &TestModel) -> Result<bool, SignalError> {
            Ok(!self.should_block)
        }
    }

    #[tokio::test]
    async fn test_middleware_blocks_signal() {
        let signal = Signal::new(SignalName::custom("test"));
        let counter = Arc::new(AtomicUsize::new(0));

        let blocking_middleware = BlockingMiddleware { should_block: true };
        signal.add_middleware(blocking_middleware);

        let counter_clone = Arc::clone(&counter);
        signal.connect(move |_instance: Arc<TestModel>| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        // Receiver should not have been called because middleware blocked it
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    struct SelectiveMiddleware {
        skip_uid: String,
    }

    #[async_trait::async_trait]
    impl SignalMiddleware<TestModel> for SelectiveMiddleware {
        async fn before_receiver(
            &self,
            _instance: &TestModel,
            dispatch_uid: Option<&str>,
        ) -> Result<bool, SignalError> {
            // Skip receiver if dispatch_uid matches
            Ok(dispatch_uid != Some(&self.skip_uid))
        }
    }

    #[tokio::test]
    async fn test_middleware_skips_specific_receiver() {
        let signal = Signal::new(SignalName::custom("test"));
        let execution_log = Arc::new(RwLock::new(Vec::new()));

        let selective_middleware = SelectiveMiddleware {
            skip_uid: "skip_me".to_string(),
        };
        signal.add_middleware(selective_middleware);

        // Receiver 1 - will be skipped
        let log_clone = Arc::clone(&execution_log);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let log = Arc::clone(&log_clone);
                async move {
                    log.write().push("receiver1");
                    Ok(())
                }
            },
            None,
            Some("skip_me".to_string()),
            0,
        );

        // Receiver 2 - will execute
        let log_clone = Arc::clone(&execution_log);
        signal.connect_with_options(
            move |_instance: Arc<TestModel>| {
                let log = Arc::clone(&log_clone);
                async move {
                    log.write().push("receiver2");
                    Ok(())
                }
            },
            None,
            Some("keep_me".to_string()),
            0,
        );

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        let log = execution_log.read();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], "receiver2");
    }

    #[tokio::test]
    async fn test_signals_multiple_middlewares() {
        let signal = Signal::new(SignalName::custom("test"));
        let log = Arc::new(RwLock::new(Vec::new()));

        // Add two logging middlewares
        let middleware1 = LoggingMiddleware {
            log: Arc::clone(&log),
        };
        let middleware2 = LoggingMiddleware {
            log: Arc::clone(&log),
        };
        signal.add_middleware(middleware1);
        signal.add_middleware(middleware2);

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 10,
            name: "Test".to_string(),
        };

        signal.send(model).await.unwrap();

        let log_entries = log.read();
        // Each middleware should have logged (2 x 4 = 8 entries)
        assert_eq!(log_entries.len(), 8);
    }

    #[allow(dead_code)]
    struct ErrorMiddleware;

    #[async_trait::async_trait]
    impl SignalMiddleware<TestModel> for ErrorMiddleware {
        async fn after_receiver(
            &self,
            _instance: &TestModel,
            _dispatch_uid: Option<&str>,
            result: &Result<(), SignalError>,
        ) -> Result<(), SignalError> {
            if result.is_err() {
                // Transform error or log it
                Ok(()) // In this test, we just swallow the error
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_middleware_with_send_robust() {
        let signal = Signal::new(SignalName::custom("test"));
        let log = Arc::new(RwLock::new(Vec::new()));

        let logging_middleware = LoggingMiddleware {
            log: Arc::clone(&log),
        };
        signal.add_middleware(logging_middleware);

        // Receiver that fails
        signal.connect(
            |_instance: Arc<TestModel>| async move { Err(SignalError::new("Test error")) },
        );

        // Receiver that succeeds
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 5,
            name: "Test".to_string(),
        };

        let results = signal.send_robust(model, None).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_err());
        assert!(results[1].is_ok());

        // Middleware should have logged all hooks
        let log_entries = log.read();
        assert!(log_entries[0].starts_with("before_send"));
        assert!(log_entries[log_entries.len() - 1].starts_with("after_send"));
    }

    // SignalSpy tests
    #[tokio::test]
    async fn test_signal_spy_basic() {
        let signal = Signal::new(SignalName::custom("test"));
        let spy = SignalSpy::new();
        signal.add_middleware(spy.clone());

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        assert!(!spy.was_called());
        assert_eq!(spy.call_count(), 0);

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        signal.send(model).await.unwrap();

        assert!(spy.was_called());
        assert_eq!(spy.call_count(), 1);
        assert_eq!(spy.total_receivers_called(), 1);
        assert!(!spy.has_errors());
    }

    #[tokio::test]
    async fn test_signal_spy_multiple_calls() {
        let signal = Signal::new(SignalName::custom("test"));
        let spy = SignalSpy::new();
        signal.add_middleware(spy.clone());

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        for i in 1..=5 {
            let model = TestModel {
                id: i,
                name: format!("Test{}", i),
            };
            signal.send(model).await.unwrap();
        }

        assert!(spy.was_called_with_count(5));
        assert_eq!(spy.total_receivers_called(), 5);
    }

    #[tokio::test]
    async fn test_signal_spy_with_errors() {
        let signal = Signal::new(SignalName::custom("test"));
        let spy = SignalSpy::new();
        signal.add_middleware(spy.clone());

        // Receiver that fails
        signal.connect(
            |_instance: Arc<TestModel>| async move { Err(SignalError::new("Test error")) },
        );

        // Receiver that succeeds
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        // Use send_robust to collect all errors
        let _ = signal.send_robust(model, None).await;

        assert!(spy.was_called());
        assert!(spy.has_errors());

        let errors = spy.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], "Test error");
    }

    #[tokio::test]
    async fn test_signal_spy_reset() {
        let signal = Signal::new(SignalName::custom("test"));
        let spy = SignalSpy::new();
        signal.add_middleware(spy.clone());

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model1 = TestModel {
            id: 1,
            name: "Test1".to_string(),
        };
        signal.send(model1).await.unwrap();

        assert_eq!(spy.call_count(), 1);

        spy.reset();

        assert_eq!(spy.call_count(), 0);
        assert!(!spy.was_called());

        let model2 = TestModel {
            id: 2,
            name: "Test2".to_string(),
        };
        signal.send(model2).await.unwrap();

        assert_eq!(spy.call_count(), 1);
    }

    #[tokio::test]
    async fn test_signal_spy_call_details() {
        let signal = Signal::new(SignalName::custom("test"));
        let spy = SignalSpy::new();
        signal.add_middleware(spy.clone());

        // Add 3 receivers
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        signal.send(model).await.unwrap();

        let calls = spy.calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].signal_sent);
        assert_eq!(calls[0].receivers_called, 3);
        assert!(calls[0].errors.is_empty());
    }

    #[tokio::test]
    async fn test_signal_spy_with_multiple_signals() {
        let signal1 = Signal::new(SignalName::custom("test1"));
        let signal2 = Signal::new(SignalName::custom("test2"));

        let spy1 = SignalSpy::new();
        let spy2 = SignalSpy::new();

        signal1.add_middleware(spy1.clone());
        signal2.add_middleware(spy2.clone());

        signal1.connect(|_instance: Arc<TestModel>| async move { Ok(()) });
        signal2.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };

        signal1.send(model.clone()).await.unwrap();
        signal2.send(model).await.unwrap();

        assert_eq!(spy1.call_count(), 1);
        assert_eq!(spy2.call_count(), 1);
    }

    // SignalContext tests
    #[tokio::test]
    async fn test_signal_context_basic() {
        let context = SignalContext::new();

        // Insert values
        context.insert("user_id", 42u64);
        context.insert("request_path", "/api/users".to_string());

        // Retrieve values
        let user_id = context.get::<u64>("user_id");
        assert!(user_id.is_some());
        assert_eq!(*user_id.unwrap(), 42);

        let path = context.get::<String>("request_path");
        assert!(path.is_some());
        assert_eq!(*path.unwrap(), "/api/users");

        // Non-existent key
        assert!(context.get::<u64>("non_existent").is_none());
    }

    #[tokio::test]
    async fn test_signal_context_contains_and_remove() {
        let context = SignalContext::new();

        context.insert("key1", 100i32);
        assert!(context.contains_key("key1"));
        assert!(!context.contains_key("key2"));

        let removed = context.remove("key1");
        assert!(removed);
        assert!(!context.contains_key("key1"));

        let not_removed = context.remove("key2");
        assert!(!not_removed);
    }

    #[tokio::test]
    async fn test_signal_context_clear() {
        let context = SignalContext::new();

        context.insert("key1", 1);
        context.insert("key2", 2);
        context.insert("key3", 3);

        assert_eq!(context.keys().len(), 3);

        context.clear();
        assert_eq!(context.keys().len(), 0);
        assert!(!context.contains_key("key1"));
    }

    #[tokio::test]
    async fn test_signal_context_clone() {
        let context1 = SignalContext::new();
        context1.insert("shared", 999);

        let context2 = context1.clone();

        // Both contexts share the same data
        let value1 = context1.get::<i32>("shared");
        let value2 = context2.get::<i32>("shared");

        assert_eq!(*value1.unwrap(), 999);
        assert_eq!(*value2.unwrap(), 999);

        // Modifications are visible to both
        context1.insert("shared", 1000);
        let value2_updated = context2.get::<i32>("shared");
        assert_eq!(*value2_updated.unwrap(), 1000);
    }

    #[tokio::test]
    async fn test_signal_with_context() {
        let signal = Signal::new(SignalName::custom("test"));

        // Access signal context
        let context = signal.context();
        context.insert("request_id", "req-123".to_string());

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        signal.send(model).await.unwrap();

        // Context data persists
        let request_id = context.get::<String>("request_id");
        assert!(request_id.is_some());
        assert_eq!(*request_id.unwrap(), "req-123");
    }

    #[derive(Clone)]
    struct UserInfo {
        user_id: u64,
        username: String,
    }

    #[tokio::test]
    async fn test_signal_context_custom_types() {
        let context = SignalContext::new();

        let user = UserInfo {
            user_id: 42,
            username: "alice".to_string(),
        };

        context.insert("current_user", user);

        let retrieved = context.get::<UserInfo>("current_user");
        assert!(retrieved.is_some());
        let user_info = retrieved.unwrap();
        assert_eq!(user_info.user_id, 42);
        assert_eq!(user_info.username, "alice");
    }

    #[tokio::test]
    async fn test_signal_context_type_safety() {
        let context = SignalContext::new();

        context.insert("value", 42i32);

        // Wrong type returns None
        assert!(context.get::<u64>("value").is_none());

        // Correct type returns Some
        assert!(context.get::<i32>("value").is_some());
    }

    // ========================================
    // Signal Composition Tests
    // ========================================

    #[tokio::test]
    async fn test_signal_chain() {
        use parking_lot::Mutex;

        let signal_a = Signal::new(SignalName::custom("signal_a"));
        let signal_b = Signal::new(SignalName::custom("signal_b"));

        let calls = Arc::new(Mutex::new(Vec::new()));

        // Track signal_a calls
        let calls_a = calls.clone();
        signal_a.connect(move |instance: Arc<TestModel>| {
            let calls = calls_a.clone();
            async move {
                calls.lock().push(format!("signal_a: {}", instance.id));
                Ok(())
            }
        });

        // Track signal_b calls
        let calls_b = calls.clone();
        signal_b.connect(move |instance: Arc<TestModel>| {
            let calls = calls_b.clone();
            async move {
                calls.lock().push(format!("signal_b: {}", instance.id));
                Ok(())
            }
        });

        // Chain signal_a to signal_b
        signal_a.chain(&signal_b);

        // Send to signal_a
        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        signal_a.send(model).await.unwrap();

        // Wait for async execution
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let results = calls.lock();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "signal_a: 1");
        assert_eq!(results[1], "signal_b: 1");
    }

    #[derive(Debug, Clone)]
    struct NotificationPayload {
        #[allow(dead_code)]
        user_id: i32,
        message: String,
    }

    #[tokio::test]
    async fn test_signal_chain_with_transform() {
        let user_signal = Signal::new(SignalName::custom("user_created"));
        let notification_signal = Signal::new(SignalName::custom("send_notification"));

        let sent_notifications = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let sent = sent_notifications.clone();
        notification_signal.connect(move |payload: Arc<NotificationPayload>| {
            let sent = sent.clone();
            async move {
                sent.lock().push(payload.message.clone());
                Ok(())
            }
        });

        // Chain with transformation
        user_signal.chain_with(&notification_signal, |user: Arc<TestModel>| {
            NotificationPayload {
                user_id: user.id,
                message: format!("Welcome, {}!", user.name),
            }
        });

        let user = TestModel {
            id: 42,
            name: "Alice".to_string(),
        };
        user_signal.send(user).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let notifications = sent_notifications.lock();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0], "Welcome, Alice!");
    }

    #[tokio::test]
    async fn test_signal_merge() {
        let signal_a = Signal::new(SignalName::custom("signal_a"));
        let signal_b = Signal::new(SignalName::custom("signal_b"));
        let signal_c = Signal::new(SignalName::custom("signal_c"));

        let merged = Signal::merge(vec![&signal_a, &signal_b, &signal_c]);

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let calls_clone = calls.clone();
        merged.connect(move |instance: Arc<TestModel>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push(instance.id);
                Ok(())
            }
        });

        // Send to each signal
        signal_a
            .send(TestModel {
                id: 1,
                name: "A".to_string(),
            })
            .await
            .unwrap();
        signal_b
            .send(TestModel {
                id: 2,
                name: "B".to_string(),
            })
            .await
            .unwrap();
        signal_c
            .send(TestModel {
                id: 3,
                name: "C".to_string(),
            })
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let results = calls.lock();
        assert_eq!(results.len(), 3);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(results.contains(&3));
    }

    #[tokio::test]
    async fn test_signal_filter() {
        let signal = Signal::new(SignalName::custom("user_signal"));
        let admin_only = signal.filter(|model: &TestModel| model.id > 100);

        let admin_calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let all_calls = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let admin = admin_calls.clone();
        admin_only.connect(move |instance: Arc<TestModel>| {
            let admin = admin.clone();
            async move {
                admin.lock().push(instance.id);
                Ok(())
            }
        });

        let all = all_calls.clone();
        signal.connect(move |instance: Arc<TestModel>| {
            let all = all.clone();
            async move {
                all.lock().push(instance.id);
                Ok(())
            }
        });

        // Send regular user (id <= 100)
        signal
            .send(TestModel {
                id: 50,
                name: "Regular".to_string(),
            })
            .await
            .unwrap();

        // Send admin user (id > 100)
        signal
            .send(TestModel {
                id: 101,
                name: "Admin".to_string(),
            })
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let admin_results = admin_calls.lock();
        let all_results = all_calls.lock();

        assert_eq!(admin_results.len(), 1); // Only admin
        assert_eq!(admin_results[0], 101);

        assert_eq!(all_results.len(), 2); // Both
    }

    #[tokio::test]
    async fn test_signal_map() {
        let user_signal = Signal::new(SignalName::custom("user_signal"));
        let id_signal: Signal<i32> = user_signal.map(|user: Arc<TestModel>| user.id);

        let ids = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let ids_clone = ids.clone();
        id_signal.connect(move |id: Arc<i32>| {
            let ids = ids_clone.clone();
            async move {
                ids.lock().push(*id);
                Ok(())
            }
        });

        user_signal
            .send(TestModel {
                id: 1,
                name: "Alice".to_string(),
            })
            .await
            .unwrap();
        user_signal
            .send(TestModel {
                id: 2,
                name: "Bob".to_string(),
            })
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let results = ids.lock();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], 1);
        assert_eq!(results[1], 2);
    }

    #[tokio::test]
    async fn test_signal_composition_complex() {
        // Test chaining filter and map together
        let user_signal = Signal::new(SignalName::custom("users"));

        // Filter for admins only
        let admin_signal = user_signal.filter(|user: &TestModel| user.id > 100);

        // Map to user IDs
        let admin_ids: Signal<i32> = admin_signal.map(|user: Arc<TestModel>| user.id);

        let ids = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let ids_clone = ids.clone();
        admin_ids.connect(move |id: Arc<i32>| {
            let ids = ids_clone.clone();
            async move {
                ids.lock().push(*id);
                Ok(())
            }
        });

        // Send various users
        user_signal
            .send(TestModel {
                id: 50,
                name: "Regular".to_string(),
            })
            .await
            .unwrap();
        user_signal
            .send(TestModel {
                id: 150,
                name: "Admin1".to_string(),
            })
            .await
            .unwrap();
        user_signal
            .send(TestModel {
                id: 200,
                name: "Admin2".to_string(),
            })
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let results = ids.lock();
        assert_eq!(results.len(), 2); // Only admins
        assert!(results.contains(&150));
        assert!(results.contains(&200));
    }

    // ========================================
    // Metrics Tests
    // ========================================

    #[tokio::test]
    async fn test_signal_metrics_basic() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        // Initial metrics should be zero
        let metrics = signal.metrics();
        assert_eq!(metrics.send_count, 0);
        assert_eq!(metrics.receiver_executions, 0);
        assert_eq!(metrics.failed_executions, 0);

        // Connect a receiver
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Send signal
        signal
            .send(TestModel {
                id: 1,
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        // Check metrics
        let metrics = signal.metrics();
        assert_eq!(metrics.send_count, 1);
        assert_eq!(metrics.receiver_executions, 1);
        assert_eq!(metrics.failed_executions, 0);
        assert_eq!(metrics.success_rate(), 100.0);
    }

    #[tokio::test]
    async fn test_signal_metrics_multiple_sends() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Send signal 5 times
        for i in 1..=5 {
            signal
                .send(TestModel {
                    id: i,
                    name: format!("Test{}", i),
                })
                .await
                .unwrap();
        }

        let metrics = signal.metrics();
        assert_eq!(metrics.send_count, 5);
        assert_eq!(metrics.receiver_executions, 5);
        assert_eq!(metrics.failed_executions, 0);
    }

    #[tokio::test]
    async fn test_signal_metrics_multiple_receivers() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        // Connect 3 receivers
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Send signal once
        signal
            .send(TestModel {
                id: 1,
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        let metrics = signal.metrics();
        assert_eq!(metrics.send_count, 1);
        assert_eq!(metrics.receiver_executions, 3); // All 3 receivers executed
        assert_eq!(metrics.failed_executions, 0);
    }

    #[tokio::test]
    async fn test_signal_metrics_with_failures() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        // Connect successful receiver
        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Connect failing receiver
        signal.connect(
            |_instance: Arc<TestModel>| async move { Err(SignalError::new("Test error")) },
        );

        // Use send_robust to continue on error
        let results = signal
            .send_robust(
                TestModel {
                    id: 1,
                    name: "Test".to_string(),
                },
                None,
            )
            .await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());

        let metrics = signal.metrics();
        assert_eq!(metrics.send_count, 1);
        assert_eq!(metrics.receiver_executions, 2);
        assert_eq!(metrics.failed_executions, 1);
        assert_eq!(metrics.success_rate(), 50.0);
    }

    #[tokio::test]
    async fn test_signal_metrics_execution_time() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        // Connect a receiver that takes some time
        signal.connect(|_instance: Arc<TestModel>| async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        });

        signal
            .send(TestModel {
                id: 1,
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        let metrics = signal.metrics();
        assert!(metrics.avg_execution_time_ns > 0);
        assert!(metrics.min_execution_time_ns > 0);
        assert!(metrics.max_execution_time_ns > 0);
        assert!(metrics.avg_execution_time().as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_signal_metrics_reset() {
        let signal = Signal::new(SignalName::custom("test_signal"));

        signal.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Send signal
        signal
            .send(TestModel {
                id: 1,
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        let metrics_before = signal.metrics();
        assert_eq!(metrics_before.send_count, 1);
        assert_eq!(metrics_before.receiver_executions, 1);

        // Reset metrics
        signal.reset_metrics();

        let metrics_after = signal.metrics();
        assert_eq!(metrics_after.send_count, 0);
        assert_eq!(metrics_after.receiver_executions, 0);
        assert_eq!(metrics_after.failed_executions, 0);
    }

    #[tokio::test]
    async fn test_signal_metrics_shared_across_clones() {
        let signal1 = Signal::new(SignalName::custom("test_signal"));
        let signal2 = signal1.clone();

        signal1.connect(|_instance: Arc<TestModel>| async move { Ok(()) });

        // Send via signal1
        signal1
            .send(TestModel {
                id: 1,
                name: "Test1".to_string(),
            })
            .await
            .unwrap();

        // Send via signal2
        signal2
            .send(TestModel {
                id: 2,
                name: "Test2".to_string(),
            })
            .await
            .unwrap();

        // Both clones should see the same metrics
        let metrics1 = signal1.metrics();
        let metrics2 = signal2.metrics();

        assert_eq!(metrics1.send_count, 2);
        assert_eq!(metrics2.send_count, 2);
        assert_eq!(metrics1.receiver_executions, metrics2.receiver_executions);
    }

    // ========================================
    // Additional Signal Types Tests
    // ========================================

    #[tokio::test]
    async fn test_m2m_changed_signal() {
        use crate::{m2m_changed, M2MAction, M2MChangeEvent};

        let signal = m2m_changed::<TestModel, TestModel>();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<M2MChangeEvent<TestModel, TestModel>>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push(event.action);
                Ok(())
            }
        });

        // Send m2m_changed signal
        let event = M2MChangeEvent::new(
            TestModel {
                id: 1,
                name: "User".to_string(),
            },
            M2MAction::PostAdd,
            vec![TestModel {
                id: 2,
                name: "Group".to_string(),
            }],
        )
        .with_reverse(false)
        .with_model_name("Group");

        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], M2MAction::PostAdd);
    }

    #[tokio::test]
    async fn test_m2m_action_display() {
        use crate::M2MAction;

        assert_eq!(M2MAction::PreAdd.to_string(), "pre_add");
        assert_eq!(M2MAction::PostAdd.to_string(), "post_add");
        assert_eq!(M2MAction::PreRemove.to_string(), "pre_remove");
        assert_eq!(M2MAction::PostRemove.to_string(), "post_remove");
        assert_eq!(M2MAction::PreClear.to_string(), "pre_clear");
        assert_eq!(M2MAction::PostClear.to_string(), "post_clear");
    }

    #[tokio::test]
    async fn test_pre_init_signal() {
        use crate::{pre_init, PreInitEvent};

        let signal = pre_init::<TestModel>();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<PreInitEvent<TestModel>>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push(event.model_type.clone());
                Ok(())
            }
        });

        let event =
            PreInitEvent::new("TestModel").with_args(vec!["arg1".to_string(), "arg2".to_string()]);
        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "TestModel");
    }

    #[tokio::test]
    async fn test_post_init_signal() {
        use crate::{post_init, PostInitEvent};

        let signal = post_init::<TestModel>();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<PostInitEvent<TestModel>>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push(event.instance.id);
                Ok(())
            }
        });

        let event = PostInitEvent::new(TestModel {
            id: 42,
            name: "Initialized".to_string(),
        });
        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 42);
    }

    #[tokio::test]
    async fn test_migration_signals() {
        use crate::{post_migrate, pre_migrate, MigrationEvent};

        let pre_signal = pre_migrate();
        let post_signal = post_migrate();

        let pre_calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let post_calls = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let pre_clone = pre_calls.clone();
        pre_signal.connect(move |event: Arc<MigrationEvent>| {
            let calls = pre_clone.clone();
            async move {
                calls.lock().push(event.app_name.clone());
                Ok(())
            }
        });

        let post_clone = post_calls.clone();
        post_signal.connect(move |event: Arc<MigrationEvent>| {
            let calls = post_clone.clone();
            async move {
                calls.lock().push(event.migration_name.clone());
                Ok(())
            }
        });

        let event =
            MigrationEvent::new("myapp", "0001_initial").with_plan(vec!["CreateModel".to_string()]);

        pre_signal.send(event.clone()).await.unwrap();
        post_signal.send(event).await.unwrap();

        assert_eq!(pre_calls.lock().len(), 1);
        assert_eq!(pre_calls.lock()[0], "myapp");
        assert_eq!(post_calls.lock().len(), 1);
        assert_eq!(post_calls.lock()[0], "0001_initial");
    }

    #[tokio::test]
    async fn test_class_prepared_signal() {
        use crate::{class_prepared, ClassPreparedEvent};

        let signal = class_prepared();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<ClassPreparedEvent>| {
            let calls = calls_clone.clone();
            async move {
                calls
                    .lock()
                    .push((event.model_name.clone(), event.app_label.clone()));
                Ok(())
            }
        });

        let event = ClassPreparedEvent::new("User", "auth");
        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ("User".to_string(), "auth".to_string()));
    }

    #[tokio::test]
    async fn test_request_signals() {
        use crate::{request_finished, request_started, RequestFinishedEvent, RequestStartedEvent};

        let started_signal = request_started();
        let finished_signal = request_finished();

        let started_calls = Arc::new(parking_lot::Mutex::new(0));
        let finished_calls = Arc::new(parking_lot::Mutex::new(0));

        let started_clone = started_calls.clone();
        started_signal.connect(move |_event: Arc<RequestStartedEvent>| {
            let calls = started_clone.clone();
            async move {
                *calls.lock() += 1;
                Ok(())
            }
        });

        let finished_clone = finished_calls.clone();
        finished_signal.connect(move |_event: Arc<RequestFinishedEvent>| {
            let calls = finished_clone.clone();
            async move {
                *calls.lock() += 1;
                Ok(())
            }
        });

        let mut environ = HashMap::new();
        environ.insert("REQUEST_METHOD".to_string(), "GET".to_string());

        started_signal
            .send(RequestStartedEvent::new().with_environ(environ.clone()))
            .await
            .unwrap();
        finished_signal
            .send(RequestFinishedEvent::new().with_environ(environ))
            .await
            .unwrap();

        assert_eq!(*started_calls.lock(), 1);
        assert_eq!(*finished_calls.lock(), 1);
    }

    #[tokio::test]
    async fn test_got_request_exception_signal() {
        use crate::{got_request_exception, GotRequestExceptionEvent};

        let signal = got_request_exception();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<GotRequestExceptionEvent>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push(event.error_message.clone());
                Ok(())
            }
        });

        let mut request_info = HashMap::new();
        request_info.insert("path".to_string(), "/api/users".to_string());

        let event = GotRequestExceptionEvent::new("Database connection failed")
            .with_request_info(request_info);
        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "Database connection failed");
    }

    #[tokio::test]
    async fn test_setting_changed_signal() {
        use crate::{setting_changed, SettingChangedEvent};

        let signal = setting_changed();

        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        signal.connect(move |event: Arc<SettingChangedEvent>| {
            let calls = calls_clone.clone();
            async move {
                calls.lock().push((
                    event.setting_name.clone(),
                    event.old_value.clone(),
                    event.new_value.clone(),
                ));
                Ok(())
            }
        });

        let event = SettingChangedEvent::new("DEBUG", Some("False".to_string()), "True");
        signal.send(event).await.unwrap();

        let results = calls.lock();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "DEBUG");
        assert_eq!(results[0].1, Some("False".to_string()));
        assert_eq!(results[0].2, "True");
    }

    #[tokio::test]
    async fn test_all_signal_types_accessible() {
        // This test ensures all signal functions are accessible and can be connected/sent
        use crate::*;
        use std::sync::{Arc, Mutex};

        // Model signals - Verify that connect/send actually work
        let pre_save_signal = pre_save::<TestModel>();
        let receiver_called = Arc::new(Mutex::new(false));
        let receiver_called_clone = receiver_called.clone();

        pre_save_signal.connect(move |_instance| {
            let called = receiver_called_clone.clone();
            async move {
                *called.lock().unwrap() = true;
                Ok(())
            }
        });

        let test_instance = TestModel {
            id: 1,
            name: "test".to_string(),
        };
        pre_save_signal.send(test_instance).await.unwrap();
        assert!(
            *receiver_called.lock().unwrap(),
            "pre_save signal receiver was not called"
        );

        // Confirm that other signals can also be generated
        let _ = post_save::<TestModel>();
        let _ = pre_delete::<TestModel>();
        let _ = post_delete::<TestModel>();
        let _ = pre_init::<TestModel>();
        let _ = post_init::<TestModel>();
        let _ = m2m_changed::<TestModel, TestModel>();

        // Migration signals
        let _ = pre_migrate();
        let _ = post_migrate();

        // Class signal
        let _ = class_prepared();

        // Request signals
        let _ = request_started();
        let _ = request_finished();
        let _ = got_request_exception();

        // Management signals
        let _ = setting_changed();
    }

    #[test]
    fn test_signal_name_validation_empty() {
        let result = SignalName::custom_validated("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Signal name cannot be empty"));
    }

    #[test]
    fn test_signal_name_validation_reserved() {
        let reserved_names = [
            "pre_save",
            "post_save",
            "pre_delete",
            "post_delete",
            "pre_init",
            "post_init",
            "m2m_changed",
            "class_prepared",
            "pre_migrate",
            "post_migrate",
            "request_started",
            "request_finished",
            "got_request_exception",
            "setting_changed",
            "db_before_insert",
            "db_after_insert",
            "db_before_update",
            "db_after_update",
            "db_before_delete",
            "db_after_delete",
        ];

        for name in &reserved_names {
            let result = SignalName::custom_validated(name);
            assert!(
                result.is_err(),
                "Reserved name '{}' should fail validation",
                name
            );
            assert!(result
                .unwrap_err()
                .message
                .contains("is reserved and cannot be used"));
        }
    }

    #[test]
    fn test_signal_name_validation_snake_case() {
        // Valid snake_case names
        let valid_names = [
            "my_custom_signal",
            "user_created",
            "order_completed",
            "signal_123",
            "test_signal_2",
            "_private_signal",
        ];

        for name in &valid_names {
            let result = SignalName::custom_validated(name);
            assert!(
                result.is_ok(),
                "Valid name '{}' should pass validation",
                name
            );
        }

        // Invalid names - not snake_case
        let invalid_names = [
            ("MySignal", "must use snake_case format"),
            ("mySignal", "must use snake_case format"),
            ("MY_SIGNAL", "must use snake_case format"),
            ("my-signal", "must use snake_case format"),
            ("my.signal", "must use snake_case format"),
            ("my signal", "must use snake_case format"),
            ("my__signal", "cannot contain consecutive underscores"),
            ("my_signal_", "cannot end with an underscore"),
            (
                "123signal",
                "must start with a lowercase letter or underscore",
            ),
        ];

        for (name, expected_error) in &invalid_names {
            let result = SignalName::custom_validated(name);
            assert!(
                result.is_err(),
                "Invalid name '{}' should fail validation",
                name
            );
            assert!(
                result.unwrap_err().message.contains(expected_error),
                "Error message should contain '{}'",
                expected_error
            );
        }
    }

    #[test]
    fn test_signal_name_reserved_names() {
        let reserved = SignalName::reserved_names();
        assert_eq!(reserved.len(), 20);
        assert!(reserved.contains(&"pre_save"));
        assert!(reserved.contains(&"post_save"));
        assert!(reserved.contains(&"db_after_delete"));
    }

    #[test]
    fn test_signal_name_custom_unvalidated() {
        // custom() should not validate
        let signal_name = SignalName::custom("InvalidName");
        assert_eq!(signal_name.as_str(), "InvalidName");

        // Even reserved names can be created with custom()
        let reserved = SignalName::custom("pre_save");
        assert_eq!(reserved.as_str(), "pre_save");
    }

    #[tokio::test]
    async fn test_signal_with_validated_custom_name() {
        let signal_name = SignalName::custom_validated("my_custom_event").unwrap();
        let signal = Signal::<TestModel>::new(signal_name);

        let called = Arc::new(parking_lot::Mutex::new(false));
        let called_clone = called.clone();

        signal.connect(move |_instance| {
            let called = called_clone.clone();
            async move {
                *called.lock() = true;
                Ok(())
            }
        });

        let instance = TestModel {
            id: 1,
            name: "test".to_string(),
        };

        signal.send(instance).await.unwrap();
        assert!(*called.lock());
    }
}
