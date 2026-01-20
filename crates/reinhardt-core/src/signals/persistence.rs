//! Signal persistence system for storing and replaying signals from durable storage
//!
//! This module provides functionality to persist signals to storage backends,
//! enabling signal replay, event sourcing, and audit trails.
//!
//! # Examples
//!
//! ```
//! use reinhardt_core::signals::persistence::{PersistentSignal, MemoryStore};
//! use reinhardt_core::signals::{Signal, SignalName};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct UserEvent {
//!     user_id: i32,
//!     action: String,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let signal = Signal::<UserEvent>::new(SignalName::custom("user_events"));
//! let store = MemoryStore::new();
//!
//! let persistent = PersistentSignal::new(signal, store);
//!
//! let event = UserEvent {
//!     user_id: 123,
//!     action: "login".to_string(),
//! };
//!
//! // Signal will be automatically persisted
//! persistent.send(event).await?;
//! # Ok(())
//! # }
//! ```

use super::error::SignalError;
use super::signal::Signal;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::SystemTime;

/// Stored signal event with metadata
///
/// Contains the signal payload along with metadata about when it was emitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSignal<T> {
	/// Unique identifier for this stored signal
	pub id: u64,
	/// Signal name
	pub signal_name: String,
	/// Timestamp when the signal was emitted
	pub timestamp: SystemTime,
	/// The signal payload
	pub payload: T,
}

impl<T> StoredSignal<T> {
	/// Create a new stored signal
	pub fn new(id: u64, signal_name: String, payload: T) -> Self {
		Self {
			id,
			signal_name,
			timestamp: SystemTime::now(),
			payload,
		}
	}
}

/// Trait for signal storage backends
///
/// Implement this trait to create custom storage backends for signal persistence.
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::persistence::{SignalStore, StoredSignal};
/// use reinhardt_core::signals::error::SignalError;
/// use async_trait::async_trait;
///
/// struct CustomStore;
///
/// #[async_trait]
/// impl<T: Send + Sync + 'static> SignalStore<T> for CustomStore {
///     async fn store(&self, signal: StoredSignal<T>) -> Result<(), SignalError> {
///         // Custom storage logic
///         Ok(())
///     }
///
///     async fn retrieve(&self, id: u64) -> Result<Option<StoredSignal<T>>, SignalError> {
///         // Custom retrieval logic
///         Ok(None)
///     }
///
///     async fn list(&self, limit: usize, offset: usize) -> Result<Vec<StoredSignal<T>>, SignalError> {
///         // Custom listing logic
///         Ok(Vec::new())
///     }
///
///     async fn count(&self) -> Result<u64, SignalError> {
///         Ok(0)
///     }
///
///     async fn clear(&self) -> Result<(), SignalError> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait SignalStore<T: Send + Sync + 'static>: Send + Sync {
	/// Store a signal
	async fn store(&self, signal: StoredSignal<T>) -> Result<(), SignalError>;

	/// Retrieve a signal by ID
	async fn retrieve(&self, id: u64) -> Result<Option<StoredSignal<T>>, SignalError>;

	/// List stored signals with pagination
	async fn list(&self, limit: usize, offset: usize) -> Result<Vec<StoredSignal<T>>, SignalError>;

	/// Count total stored signals
	async fn count(&self) -> Result<u64, SignalError>;

	/// Clear all stored signals
	async fn clear(&self) -> Result<(), SignalError>;
}

/// In-memory signal store for testing and development
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::persistence::MemoryStore;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct Event {
///     name: String,
/// }
///
/// let store = MemoryStore::<Event>::new();
/// assert_eq!(store.max_size(), usize::MAX);
/// ```
pub struct MemoryStore<T> {
	signals: Arc<RwLock<VecDeque<StoredSignal<T>>>>,
	next_id: Arc<RwLock<u64>>,
	max_size: usize,
}

impl<T> MemoryStore<T> {
	/// Create a new memory store with unlimited size
	pub fn new() -> Self {
		Self {
			signals: Arc::new(RwLock::new(VecDeque::new())),
			next_id: Arc::new(RwLock::new(1)),
			max_size: usize::MAX,
		}
	}

	/// Create a new memory store with a maximum size
	///
	/// When the maximum size is reached, oldest signals are evicted.
	pub fn with_max_size(max_size: usize) -> Self {
		Self {
			signals: Arc::new(RwLock::new(VecDeque::new())),
			next_id: Arc::new(RwLock::new(1)),
			max_size,
		}
	}

	/// Get the maximum size of the store
	pub fn max_size(&self) -> usize {
		self.max_size
	}
}

impl<T> Default for MemoryStore<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Clone for MemoryStore<T> {
	fn clone(&self) -> Self {
		Self {
			signals: Arc::clone(&self.signals),
			next_id: Arc::clone(&self.next_id),
			max_size: self.max_size,
		}
	}
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> SignalStore<T> for MemoryStore<T> {
	async fn store(&self, signal: StoredSignal<T>) -> Result<(), SignalError> {
		let mut signals = self.signals.write();

		// Evict oldest if at capacity
		if signals.len() >= self.max_size {
			signals.pop_front();
		}

		signals.push_back(signal);
		Ok(())
	}

	async fn retrieve(&self, id: u64) -> Result<Option<StoredSignal<T>>, SignalError> {
		let signals = self.signals.read();
		Ok(signals.iter().find(|s| s.id == id).cloned())
	}

	async fn list(&self, limit: usize, offset: usize) -> Result<Vec<StoredSignal<T>>, SignalError> {
		let signals = self.signals.read();
		Ok(signals.iter().skip(offset).take(limit).cloned().collect())
	}

	async fn count(&self) -> Result<u64, SignalError> {
		Ok(self.signals.read().len() as u64)
	}

	async fn clear(&self) -> Result<(), SignalError> {
		self.signals.write().clear();
		Ok(())
	}
}

/// Persistent signal wrapper that stores signals to a backend
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::persistence::{PersistentSignal, MemoryStore};
/// use reinhardt_core::signals::{Signal, SignalName};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct Event {
///     id: i32,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let signal = Signal::<Event>::new(SignalName::custom("events"));
/// let store = MemoryStore::new();
/// let persistent = PersistentSignal::new(signal, store);
///
/// persistent.send(Event { id: 1 }).await?;
/// # Ok(())
/// # }
/// ```
pub struct PersistentSignal<T: Send + Sync + 'static> {
	signal: Signal<T>,
	store: Arc<dyn SignalStore<T>>,
	signal_name: String,
	next_id: Arc<RwLock<u64>>,
}

impl<T: Send + Sync + Clone + 'static> PersistentSignal<T> {
	/// Create a new persistent signal
	///
	/// # Arguments
	///
	/// * `signal` - The underlying signal to wrap
	/// * `store` - The storage backend to use
	pub fn new<S>(signal: Signal<T>, store: S) -> Self
	where
		S: SignalStore<T> + 'static,
	{
		let signal_name = format!("persistent_{}", std::any::type_name::<T>());

		Self {
			signal,
			store: Arc::new(store),
			signal_name,
			next_id: Arc::new(RwLock::new(1)),
		}
	}

	/// Send a signal and persist it
	///
	/// The signal will be sent to all receivers and also stored in the backend.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_core::signals::persistence::{PersistentSignal, MemoryStore};
	/// # use reinhardt_core::signals::{Signal, SignalName};
	/// # use serde::{Deserialize, Serialize};
	/// # #[derive(Clone, Serialize, Deserialize)]
	/// # struct Event { id: i32 }
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let signal = Signal::<Event>::new(SignalName::custom("events"));
	/// # let store = MemoryStore::new();
	/// # let persistent = PersistentSignal::new(signal, store);
	/// persistent.send(Event { id: 42 }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn send(&self, instance: T) -> Result<(), SignalError> {
		// Clone for storage
		let stored_instance = instance.clone();

		// Generate ID and create stored signal
		let id = {
			let mut next_id = self.next_id.write();
			let id = *next_id;
			*next_id += 1;
			id
		};

		let stored_signal = StoredSignal::new(id, self.signal_name.clone(), stored_instance);

		// Store first
		self.store.store(stored_signal).await?;

		// Then send to receivers
		self.signal.send(instance).await
	}

	/// Retrieve a stored signal by ID
	pub async fn retrieve(&self, id: u64) -> Result<Option<StoredSignal<T>>, SignalError> {
		self.store.retrieve(id).await
	}

	/// List stored signals with pagination
	pub async fn list(
		&self,
		limit: usize,
		offset: usize,
	) -> Result<Vec<StoredSignal<T>>, SignalError> {
		self.store.list(limit, offset).await
	}

	/// Count total stored signals
	pub async fn count(&self) -> Result<u64, SignalError> {
		self.store.count().await
	}

	/// Clear all stored signals
	pub async fn clear(&self) -> Result<(), SignalError> {
		self.store.clear().await
	}

	/// Get access to the underlying signal
	pub fn signal(&self) -> &Signal<T> {
		&self.signal
	}

	/// Get access to the storage backend
	pub fn store(&self) -> Arc<dyn SignalStore<T>> {
		Arc::clone(&self.store)
	}
}

impl<T: Send + Sync + Clone + 'static> Clone for PersistentSignal<T> {
	fn clone(&self) -> Self {
		Self {
			signal: self.signal.clone(),
			store: Arc::clone(&self.store),
			signal_name: self.signal_name.clone(),
			next_id: Arc::clone(&self.next_id),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::signals::SignalName;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestEvent {
		id: i32,
		message: String,
	}

	#[tokio::test]
	async fn test_memory_store_basic() {
		let store = MemoryStore::new();
		let event = StoredSignal::new(
			1,
			"test".to_string(),
			TestEvent {
				id: 1,
				message: "Hello".to_string(),
			},
		);

		// Store
		store.store(event.clone()).await.unwrap();

		// Retrieve
		let retrieved = store.retrieve(1).await.unwrap();
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().payload.id, 1);

		// Count
		assert_eq!(store.count().await.unwrap(), 1);

		// List
		let list = store.list(10, 0).await.unwrap();
		assert_eq!(list.len(), 1);
		assert_eq!(list[0].payload.message, "Hello");
	}

	#[tokio::test]
	async fn test_memory_store_max_size() {
		let store = MemoryStore::with_max_size(3);

		// Store 5 events
		for i in 1..=5 {
			let event = StoredSignal::new(
				i,
				"test".to_string(),
				TestEvent {
					id: i as i32,
					message: format!("Event {}", i),
				},
			);
			store.store(event).await.unwrap();
		}

		// Should only have 3 events (oldest evicted)
		assert_eq!(store.count().await.unwrap(), 3);

		// Should have events 3, 4, 5
		let list = store.list(10, 0).await.unwrap();
		assert_eq!(list[0].id, 3);
		assert_eq!(list[1].id, 4);
		assert_eq!(list[2].id, 5);
	}

	#[tokio::test]
	async fn test_memory_store_clear() {
		let store = MemoryStore::new();

		for i in 1..=3 {
			let event = StoredSignal::new(
				i,
				"test".to_string(),
				TestEvent {
					id: i as i32,
					message: "test".to_string(),
				},
			);
			store.store(event).await.unwrap();
		}

		assert_eq!(store.count().await.unwrap(), 3);

		store.clear().await.unwrap();
		assert_eq!(store.count().await.unwrap(), 0);
	}

	#[tokio::test]
	async fn test_persistent_signal_send_and_store() {
		let signal = Signal::<TestEvent>::new(SignalName::custom("test_persistent"));
		let store = MemoryStore::new();
		let persistent = PersistentSignal::new(signal.clone(), store.clone());

		let counter = Arc::new(AtomicUsize::new(0));
		let counter_clone = Arc::clone(&counter);

		signal.connect(move |_event| {
			let counter = Arc::clone(&counter_clone);
			async move {
				counter.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}
		});

		let event = TestEvent {
			id: 42,
			message: "Test event".to_string(),
		};

		// Send through persistent signal
		persistent.send(event.clone()).await.unwrap();

		// Wait for processing

		// Verify signal was sent to receivers
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Verify signal was stored
		assert_eq!(store.count().await.unwrap(), 1);
		let stored = store.retrieve(1).await.unwrap();
		assert!(stored.is_some());
		assert_eq!(stored.unwrap().payload.id, 42);
	}

	#[tokio::test]
	async fn test_persistent_signal_list_pagination() {
		let signal = Signal::<TestEvent>::new(SignalName::custom("test_pagination"));
		let store = MemoryStore::new();
		let persistent = PersistentSignal::new(signal, store);

		// Send multiple events
		for i in 1..=10 {
			let event = TestEvent {
				id: i,
				message: format!("Event {}", i),
			};
			persistent.send(event).await.unwrap();
		}

		// Test pagination
		let page1 = persistent.list(5, 0).await.unwrap();
		assert_eq!(page1.len(), 5);
		assert_eq!(page1[0].payload.id, 1);

		let page2 = persistent.list(5, 5).await.unwrap();
		assert_eq!(page2.len(), 5);
		assert_eq!(page2[0].payload.id, 6);

		// Test count
		assert_eq!(persistent.count().await.unwrap(), 10);
	}

	#[tokio::test]
	async fn test_persistent_signal_clear() {
		let signal = Signal::<TestEvent>::new(SignalName::custom("test_clear"));
		let store = MemoryStore::new();
		let persistent = PersistentSignal::new(signal, store);

		for i in 1..=5 {
			persistent
				.send(TestEvent {
					id: i,
					message: "test".to_string(),
				})
				.await
				.unwrap();
		}

		assert_eq!(persistent.count().await.unwrap(), 5);

		persistent.clear().await.unwrap();
		assert_eq!(persistent.count().await.unwrap(), 0);
	}
}
