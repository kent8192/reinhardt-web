//! Key marker trait for keyed dependency providers.

/// Marker trait for dependency provider keys.
///
/// A key names one provider slot. Multiple providers may produce the same
/// value type as long as they use different key types.
pub trait InjectableKey: Send + Sync + 'static {}
