//! Default DI key derived from the produced value type.

use crate::InjectableKey;
use std::marker::PhantomData;

/// Default key for self-keyed dependency providers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SelfKey<T>
where
	T: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> SelfKey<T>
where
	T: Send + Sync + 'static,
{
	/// Create a marker value for diagnostics and tests.
	pub const fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T> InjectableKey for SelfKey<T> where T: Send + Sync + 'static {}
