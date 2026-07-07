//! Output wrapper for explicitly keyed injectable provider functions.

use crate::InjectableKey;
use std::marker::PhantomData;
use std::ops::Deref;

/// Registered output of an explicitly keyed provider function.
///
/// The DI registry keys this type by `TypeId::of::<KeyedFactoryOutput<K, T>>()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedFactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	value: T,
	_key: PhantomData<fn() -> K>,
}

impl<K, T> KeyedFactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Build a keyed provider output from its value.
	pub fn new(value: T) -> Self {
		Self {
			value,
			_key: PhantomData,
		}
	}

	/// Consume the output and return the wrapped value.
	pub fn into_inner(self) -> T {
		self.value
	}
}

impl<K, T> Deref for KeyedFactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<K, T> AsRef<T> for KeyedFactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn as_ref(&self) -> &T {
		&self.value
	}
}

/// Deprecated compatibility alias for the old explicitly keyed provider output.
#[deprecated(
	since = "0.4.0",
	note = "use KeyedFactoryOutput<K, T> for explicit keys, or return T directly from #[injectable] for self-keyed providers"
)]
pub type FactoryOutput<K, T> = KeyedFactoryOutput<K, T>;
