//! Global signal registry

use crate::core::SignalName;
use crate::signal::Signal;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;

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
