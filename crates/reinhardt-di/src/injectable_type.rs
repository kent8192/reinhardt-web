//! Trait-based wrapper resolution for `#[inject]`.

use crate::{
	Depends, DiResult, Injectable, InjectableKey, KeyedDepends, KeyedFactoryOutput, SelfKey,
	context::InjectionContext,
};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Marker trait for wrapper types that can be resolved by `#[inject]`.
///
/// Implement this trait for custom dependency wrappers when the wrapper should
/// resolve a registry value by an inner key type instead of being injected as
/// a normal dependency itself.
///
/// # Example
///
/// ```rust
/// use reinhardt_di::{InjectableKey, InjectableType, KeyedDepends, KeyedFactoryOutput};
/// use std::sync::Arc;
///
/// struct ConfigKey;
///
/// impl InjectableKey for ConfigKey {}
///
/// struct Lazy<K, T>(KeyedDepends<K, T>)
/// where
///     K: InjectableKey,
///     T: Send + Sync + 'static;
///
/// impl<K, T> InjectableType for Lazy<K, T>
/// where
///     K: InjectableKey,
///     T: Send + Sync + 'static,
/// {
///     type Inner = KeyedFactoryOutput<K, T>;
///
///     fn from_resolved(output: Arc<Self::Inner>, use_cache: bool) -> Self {
///         Self(KeyedDepends::from_output(output, use_cache))
///     }
/// }
/// ```
pub trait InjectableType: Sized + Send + 'static {
	/// Registry key resolved before constructing the wrapper.
	type Inner: Send + Sync + 'static;

	/// Build the wrapper from the resolved dependency handle.
	fn from_resolved(inner: Arc<Self::Inner>, use_cache: bool) -> Self;
}

impl<T> InjectableType for Depends<T>
where
	T: Send + Sync + 'static,
{
	type Inner = KeyedFactoryOutput<SelfKey<T>, T>;

	fn from_resolved(output: Arc<Self::Inner>, use_cache: bool) -> Self {
		Depends::from_output(output, use_cache)
	}
}

impl<K, T> InjectableType for KeyedDepends<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	type Inner = KeyedFactoryOutput<K, T>;

	fn from_resolved(output: Arc<Self::Inner>, use_cache: bool) -> Self {
		KeyedDepends::from_output(output, use_cache)
	}
}

#[doc(hidden)]
pub type __InjectResolveFuture<'a, T> = Pin<Box<dyn Future<Output = DiResult<T>> + Send + 'a>>;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct __InjectResolver<T> {
	_marker: PhantomData<fn() -> T>,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct __InjectDependsResolver<K, T> {
	_marker: PhantomData<fn() -> (K, T)>,
}

impl<T> __InjectResolver<T> {
	pub const fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<K, T> __InjectDependsResolver<K, T> {
	pub const fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

#[doc(hidden)]
pub trait __InjectWrapperResolver<T>
where
	T: InjectableType,
{
	fn __resolve_inject_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, T>
	where
		T: 'a;
}

impl<T> __InjectWrapperResolver<T> for __InjectResolver<T>
where
	T: InjectableType,
{
	fn __resolve_inject_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, T>
	where
		T: 'a,
	{
		Box::pin(async move {
			let inner = ctx.resolve_with_cache::<T::Inner>(use_cache).await?;
			Ok(T::from_resolved(inner, use_cache))
		})
	}
}

#[doc(hidden)]
pub trait __InjectDependsFallbackResolver<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, KeyedDepends<K, T>>
	where
		T: 'a;
}

impl<K, T> __InjectDependsFallbackResolver<K, T> for __InjectDependsResolver<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, KeyedDepends<K, T>>
	where
		T: 'a,
	{
		Box::pin(async move { KeyedDepends::<K, T>::resolve_from_registry(ctx, use_cache).await })
	}
}

#[doc(hidden)]
pub trait __InjectDependsRegistryResolver<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, KeyedDepends<K, T>>
	where
		T: 'a;
}

impl<K, T> __InjectDependsRegistryResolver<K, T> for &__InjectDependsResolver<K, T>
where
	K: InjectableKey,
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, KeyedDepends<K, T>>
	where
		T: 'a,
	{
		Box::pin(async move { KeyedDepends::<K, T>::resolve_from_registry(ctx, use_cache).await })
	}
}

#[doc(hidden)]
pub trait __InjectFallbackResolver<T>
where
	T: Injectable + Clone,
{
	fn __resolve_inject_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, T>
	where
		T: 'a;
}

impl<T> __InjectFallbackResolver<T> for &__InjectResolver<T>
where
	T: Injectable + Clone,
{
	fn __resolve_inject_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, T>
	where
		T: 'a,
	{
		Box::pin(async move {
			if use_cache {
				match ctx.resolve::<T>().await {
					Ok(value) => Ok(value.as_ref().clone()),
					Err(crate::DiError::DependencyNotRegistered { .. }) => T::inject(ctx).await,
					Err(err) => Err(err),
				}
			} else {
				T::inject_uncached(ctx).await
			}
		})
	}
}
