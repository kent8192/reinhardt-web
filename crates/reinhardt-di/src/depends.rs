//! Depends wrapper for keyed dependency injection.

use crate::{
	DiResult, FactoryOutput, InjectableKey, context::InjectionContext, injected::DependencyScope,
	injected::InjectionMetadata,
};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

/// Dependency injection wrapper for keyed provider output.
///
/// `Depends<K, T>` resolves `FactoryOutput<K, T>` from the DI registry, while
/// dereferencing to the wrapped `T` for handler ergonomics.
#[derive(Debug)]
pub struct Depends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	inner: Arc<FactoryOutput<K, T>>,
	metadata: InjectionMetadata,
}

impl<K, T> Depends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Create a builder that records `cached = true` in dependency metadata.
	pub fn builder() -> DependsBuilder<K, T> {
		DependsBuilder {
			use_cache: true,
			_phantom: PhantomData,
		}
	}

	/// Create a builder that records `cached = false` in dependency metadata.
	///
	/// Scope cache behavior is still controlled by the registered provider scope.
	pub fn builder_no_cache() -> DependsBuilder<K, T> {
		DependsBuilder {
			use_cache: false,
			_phantom: PhantomData,
		}
	}

	/// Resolve the keyed provider output from the DI registry.
	pub async fn resolve_from_registry(ctx: &InjectionContext, use_cache: bool) -> DiResult<Self> {
		let output = ctx.resolve::<FactoryOutput<K, T>>().await?;

		Ok(Self::from_output(output, use_cache))
	}

	/// Create a `Depends` from an existing value.
	pub fn from_value(value: T) -> Self {
		Self::from_output(Arc::new(FactoryOutput::new(value)), false)
	}

	/// Create a `Depends` from an existing keyed provider output.
	pub fn from_output(output: Arc<FactoryOutput<K, T>>, use_cache: bool) -> Self {
		Self {
			inner: output,
			metadata: InjectionMetadata {
				scope: DependencyScope::Request,
				cached: use_cache,
			},
		}
	}

	/// Get the keyed provider output.
	pub fn as_output(&self) -> &FactoryOutput<K, T> {
		self.inner.as_ref()
	}

	/// Get the shared keyed provider output handle.
	pub fn as_arc(&self) -> &Arc<FactoryOutput<K, T>> {
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

impl<K, T> Depends<K, T>
where
	K: InjectableKey,
	T: Clone + Send + Sync + 'static,
{
	/// Extract the inner value, cloning when the output is shared.
	pub fn into_inner(self) -> T {
		Arc::try_unwrap(self.inner)
			.map(FactoryOutput::into_inner)
			.unwrap_or_else(|output| output.as_ref().as_ref().clone())
	}
}

/// Builder for `Depends` with a metadata cache flag recorded on the resolved wrapper.
pub struct DependsBuilder<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Metadata cache flag recorded on the resolved wrapper.
	use_cache: bool,
	_phantom: PhantomData<fn() -> (K, T)>,
}

impl<K, T> DependsBuilder<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	/// Resolve the keyed dependency.
	pub async fn resolve(self, ctx: &InjectionContext) -> DiResult<Depends<K, T>> {
		Depends::<K, T>::resolve_from_registry(ctx, self.use_cache).await
	}
}

impl<K, T> Deref for Depends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.inner.as_ref().as_ref()
	}
}

impl<K, T> Clone for Depends<K, T>
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

impl<K, T> AsRef<T> for Depends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn as_ref(&self) -> &T {
		self.inner.as_ref().as_ref()
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
	fn from_value_wraps_factory_output() {
		let depends = Depends::<TestKey, TestConfig>::from_value(TestConfig {
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
	fn from_output_preserves_cache_metadata() {
		let output = Arc::new(FactoryOutput::<TestKey, TestConfig>::new(TestConfig {
			value: "shared".to_string(),
		}));
		let depends = Depends::from_output(Arc::clone(&output), true);

		assert_eq!(depends.value, "shared");
		assert!(depends.metadata().cached);
		assert_eq!(depends.metadata().scope, DependencyScope::Request);
		assert!(Arc::ptr_eq(depends.as_arc(), &output));
	}

	#[test]
	fn try_unwrap_returns_value_for_single_owner() {
		let depends = Depends::<TestKey, TestConfig>::from_value(TestConfig {
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
		let depends = Depends::<TestKey, TestConfig>::from_value(TestConfig {
			value: "shared".to_string(),
		});
		let _clone = depends.clone();

		let returned = depends.try_unwrap().unwrap_err();

		assert_eq!(returned.value, "shared");
		assert_eq!(returned.metadata().scope, DependencyScope::Request);
	}

	#[test]
	fn into_inner_clones_shared_output() {
		let depends = Depends::<TestKey, TestConfig>::from_value(TestConfig {
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
