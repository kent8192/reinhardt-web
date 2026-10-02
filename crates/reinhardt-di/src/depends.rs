//! Dependency wrappers for self-keyed and explicitly keyed provider output.

use crate::{
	DiResult, InjectableKey, KeyedFactoryOutput, SelfKey, context::InjectionContext,
	injected::DependencyScope, injected::InjectionMetadata,
};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

/// Dependency injection wrapper for explicitly keyed provider output.
///
/// `KeyedDepends<K, T>` resolves `KeyedFactoryOutput<K, T>` from the DI
/// registry, while dereferencing to the wrapped `T` for handler ergonomics.
#[derive(Debug)]
pub struct KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	inner: Arc<KeyedFactoryOutput<K, T>>,
	metadata: InjectionMetadata,
}

impl<K, T> KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Create a builder that records `cached = true` in dependency metadata.
	pub fn builder() -> KeyedDependsBuilder<K, T> {
		KeyedDependsBuilder {
			use_cache: true,
			_phantom: PhantomData,
		}
	}

	/// Create a builder that records `cached = false` in dependency metadata.
	///
	/// This bypasses scope-cache lookup for this resolution.
	pub fn builder_no_cache() -> KeyedDependsBuilder<K, T> {
		KeyedDependsBuilder {
			use_cache: false,
			_phantom: PhantomData,
		}
	}

	/// Resolve the keyed provider output from the DI registry.
	pub async fn resolve_from_registry(ctx: &InjectionContext, use_cache: bool) -> DiResult<Self> {
		let output = ctx
			.resolve_with_cache::<KeyedFactoryOutput<K, T>>(use_cache)
			.await?;

		Ok(Self::from_output(output, use_cache))
	}

	/// Create a `KeyedDepends` from an existing value.
	pub fn from_value(value: T) -> Self {
		Self::from_output(Arc::new(KeyedFactoryOutput::new(value)), false)
	}

	/// Create a `KeyedDepends` from an existing keyed provider output.
	pub fn from_output(output: Arc<KeyedFactoryOutput<K, T>>, use_cache: bool) -> Self {
		Self {
			inner: output,
			metadata: InjectionMetadata {
				scope: DependencyScope::Request,
				cached: use_cache,
			},
		}
	}

	/// Get the keyed provider output.
	pub fn as_output(&self) -> &KeyedFactoryOutput<K, T> {
		self.inner.as_ref()
	}

	/// Get the shared keyed provider output handle.
	pub fn as_arc(&self) -> &Arc<KeyedFactoryOutput<K, T>> {
		&self.inner
	}

	/// Get injection metadata.
	pub fn metadata(&self) -> &InjectionMetadata {
		&self.metadata
	}

	/// Attempt to unwrap the inner value, returning `Self` if shared.
	pub fn try_unwrap(self) -> Result<T, Self> {
		match Arc::try_unwrap(self.inner) {
			Ok(output) => Ok(output.into_inner()),
			Err(output) => Err(Self {
				inner: output,
				metadata: self.metadata,
			}),
		}
	}
}

impl<K, T> KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Clone + Send + Sync + 'static,
{
	/// Extract the inner value, cloning when the output is shared.
	pub fn into_inner(self) -> T {
		Arc::try_unwrap(self.inner)
			.map(KeyedFactoryOutput::into_inner)
			.unwrap_or_else(|output| output.as_ref().as_ref().clone())
	}
}

/// Builder for `KeyedDepends` with a metadata cache flag recorded on the resolved wrapper.
pub struct KeyedDependsBuilder<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Metadata cache flag recorded on the resolved wrapper.
	use_cache: bool,
	_phantom: PhantomData<fn() -> (K, T)>,
}

impl<K, T> KeyedDependsBuilder<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Resolve the keyed dependency.
	pub async fn resolve(self, ctx: &InjectionContext) -> DiResult<KeyedDepends<K, T>> {
		KeyedDepends::<K, T>::resolve_from_registry(ctx, self.use_cache).await
	}
}

impl<K, T> Deref for KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.inner.as_ref().as_ref()
	}
}

impl<K, T> Clone for KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner),
			metadata: self.metadata,
		}
	}
}

impl<K, T> AsRef<T> for KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn as_ref(&self) -> &T {
		self.inner.as_ref().as_ref()
	}
}

/// Dependency injection wrapper for the default self-keyed provider output.
///
/// `Depends<T>` resolves `KeyedFactoryOutput<SelfKey<T>, T>` from the DI
/// registry.
#[derive(Debug)]
pub struct Depends<T>
where
	T: Send + Sync + 'static,
{
	inner: KeyedDepends<SelfKey<T>, T>,
}

impl<T> Depends<T>
where
	T: Send + Sync + 'static,
{
	/// Create a builder that records `cached = true` in dependency metadata.
	pub fn builder() -> DependsBuilder<T> {
		DependsBuilder {
			use_cache: true,
			_phantom: PhantomData,
		}
	}

	/// Create a builder that records `cached = false` in dependency metadata.
	///
	/// This bypasses scope-cache lookup for this resolution.
	pub fn builder_no_cache() -> DependsBuilder<T> {
		DependsBuilder {
			use_cache: false,
			_phantom: PhantomData,
		}
	}

	/// Resolve the self-keyed provider output from the DI registry.
	pub async fn resolve_from_registry(ctx: &InjectionContext, use_cache: bool) -> DiResult<Self> {
		let inner = KeyedDepends::<SelfKey<T>, T>::resolve_from_registry(ctx, use_cache).await?;
		Ok(Self { inner })
	}

	/// Create a `Depends` from an existing value.
	pub fn from_value(value: T) -> Self {
		Self {
			inner: KeyedDepends::from_value(value),
		}
	}

	/// Create a `Depends` from an existing self-keyed provider output.
	pub fn from_output(output: Arc<KeyedFactoryOutput<SelfKey<T>, T>>, use_cache: bool) -> Self {
		Self {
			inner: KeyedDepends::from_output(output, use_cache),
		}
	}

	/// Get the self-keyed provider output.
	pub fn as_output(&self) -> &KeyedFactoryOutput<SelfKey<T>, T> {
		self.inner.as_output()
	}

	/// Get the shared self-keyed provider output handle.
	pub fn as_arc(&self) -> &Arc<KeyedFactoryOutput<SelfKey<T>, T>> {
		self.inner.as_arc()
	}

	/// Get injection metadata.
	pub fn metadata(&self) -> &InjectionMetadata {
		self.inner.metadata()
	}

	/// Attempt to unwrap the inner value, returning `Self` if shared.
	pub fn try_unwrap(self) -> Result<T, Self> {
		self.inner.try_unwrap().map_err(|inner| Self { inner })
	}
}

impl<T> Depends<T>
where
	T: Clone + Send + Sync + 'static,
{
	/// Extract the inner value, cloning when the output is shared.
	pub fn into_inner(self) -> T {
		self.inner.into_inner()
	}
}

/// Builder for `Depends` with a metadata cache flag recorded on the resolved wrapper.
pub struct DependsBuilder<T>
where
	T: Send + Sync + 'static,
{
	/// Metadata cache flag recorded on the resolved wrapper.
	use_cache: bool,
	_phantom: PhantomData<fn() -> T>,
}

impl<T> DependsBuilder<T>
where
	T: Send + Sync + 'static,
{
	/// Resolve the self-keyed dependency.
	pub async fn resolve(self, ctx: &InjectionContext) -> DiResult<Depends<T>> {
		Depends::<T>::resolve_from_registry(ctx, self.use_cache).await
	}
}

impl<T> Deref for Depends<T>
where
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.inner.as_ref()
	}
}

impl<T> Clone for Depends<T>
where
	T: Send + Sync + 'static,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<T> AsRef<T> for Depends<T>
where
	T: Send + Sync + 'static,
{
	fn as_ref(&self) -> &T {
		self.inner.as_ref()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug)]
	struct TestKey;

	impl InjectableKey for TestKey {}

	#[derive(Clone, Debug, PartialEq, Eq)]
	struct TestConfig {
		value: String,
	}

	#[test]
	fn from_value_wraps_self_keyed_factory_output() {
		let depends = Depends::<TestConfig>::from_value(TestConfig {
			value: "custom".to_string(),
		});

		assert_eq!(depends.value, "custom");
		assert_eq!(
			depends.as_output().as_ref(),
			&TestConfig {
				value: "custom".to_string(),
			}
		);
		assert!(!depends.metadata().cached);
	}

	#[test]
	fn keyed_from_value_wraps_factory_output() {
		let depends = KeyedDepends::<TestKey, TestConfig>::from_value(TestConfig {
			value: "custom".to_string(),
		});

		assert_eq!(depends.value, "custom");
		assert_eq!(
			depends.as_output().as_ref(),
			&TestConfig {
				value: "custom".to_string(),
			}
		);
		assert!(!depends.metadata().cached);
	}

	#[test]
	fn keyed_from_output_preserves_cache_metadata() {
		let output = Arc::new(KeyedFactoryOutput::<TestKey, TestConfig>::new(TestConfig {
			value: "shared".to_string(),
		}));
		let depends = KeyedDepends::from_output(Arc::clone(&output), true);

		assert_eq!(depends.value, "shared");
		assert!(depends.metadata().cached);
		assert_eq!(depends.metadata().scope, DependencyScope::Request);
		assert!(Arc::ptr_eq(depends.as_arc(), &output));
	}

	#[test]
	fn try_unwrap_returns_value_for_single_owner() {
		let depends = Depends::<TestConfig>::from_value(TestConfig {
			value: "owned".to_string(),
		});

		assert_eq!(
			depends.try_unwrap().unwrap(),
			TestConfig {
				value: "owned".to_string(),
			}
		);
	}

	#[test]
	fn try_unwrap_returns_self_for_shared_output() {
		let depends = Depends::<TestConfig>::from_value(TestConfig {
			value: "shared".to_string(),
		});
		let _clone = depends.clone();

		let returned = depends.try_unwrap().unwrap_err();

		assert_eq!(returned.value, "shared");
		assert_eq!(returned.metadata().scope, DependencyScope::Request);
	}

	#[test]
	fn into_inner_clones_shared_output() {
		let depends = Depends::<TestConfig>::from_value(TestConfig {
			value: "cloned".to_string(),
		});
		let cloned = depends.clone();

		assert_eq!(
			depends.into_inner(),
			TestConfig {
				value: "cloned".to_string(),
			}
		);
		assert_eq!(cloned.value, "cloned");
	}
}
