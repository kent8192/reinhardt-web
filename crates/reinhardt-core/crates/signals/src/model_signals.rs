//! Model lifecycle signals
//!
//! Provides convenience functions for common model signals.

use crate::core::SignalName;
use crate::registry::get_signal;
use crate::signal::Signal;

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
