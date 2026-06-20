//! Output wrapper for keyed injectable provider functions.

use crate::InjectableKey;
use std::marker::PhantomData;
use std::ops::Deref;

/// Registered output of a keyed provider function.
///
/// The DI registry keys this type by `TypeId::of::<FactoryOutput<K, T>>()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	value: T,
	_key: PhantomData<fn() -> K>,
}

impl<K, T> FactoryOutput<K, T>
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

impl<K, T> Deref for FactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<K, T> AsRef<T> for FactoryOutput<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn as_ref(&self) -> &T {
		&self.value
	}
}
