//! Canonical page event names and metadata.
//!
//! Runtime event storage and standard event compatibility both consume the
//! dependency-free event catalog so parsing, hydration, and rendering cannot
//! drift onto separate name tables.

pub use reinhardt_event_catalog::{
	EventBehavior, EventInterface, EventName, KnownEvent as EventType, UnknownEventName, event_spec,
};
