//! Trait-based wrapper resolution for `#[inject]`.

use crate::{Depends, DiResult, Injectable, context::InjectionContext};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

/// Marker trait for wrapper types that can be resolved by `#[inject]`.
///
/// Implement this trait for custom dependency wrappers when the wrapper should
/// resolve a registry value by an inner key type instead of being injected as
/// a normal dependency itself.
///
/// # Example
///
/// ```rust
/// use reinhardt_di::{Depends, InjectableType};
///
/// struct Lazy<T>(Depends<T>)
/// where
///     T: Send + Sync + 'static;
///
/// impl<T> InjectableType for Lazy<T>
/// where
///     T: Send + Sync + 'static,
/// {
///     type Inner = T;
///
///     fn from_depends(depends: Depends<Self::Inner>) -> Self {
///         Self(depends)
///     }
/// }
/// ```
pub trait InjectableType: Sized + Send + 'static {
	/// Registry key resolved before constructing the wrapper.
	type Inner: Send + Sync + 'static;

	/// Build the wrapper from the resolved dependency handle.
	fn from_depends(depends: Depends<Self::Inner>) -> Self;
}

impl<T> InjectableType for Depends<T>
where
	T: Send + Sync + 'static,
{
	type Inner = T;

	fn from_depends(depends: Depends<Self::Inner>) -> Self {
		depends
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
pub struct __InjectDependsResolver<T> {
	_marker: PhantomData<fn() -> T>,
}

impl<T> __InjectResolver<T> {
	pub const fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T> __InjectDependsResolver<T> {
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
			let depends = Depends::<T::Inner>::resolve_from_registry(ctx, use_cache).await?;
			Ok(T::from_depends(depends))
		})
	}
}

#[doc(hidden)]
pub trait __InjectDependsFallbackResolver<T>
where
	T: Injectable,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, Depends<T>>
	where
		T: 'a;
}

impl<T> __InjectDependsFallbackResolver<T> for __InjectDependsResolver<T>
where
	T: Injectable,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, Depends<T>>
	where
		T: 'a,
	{
		Box::pin(async move { Depends::<T>::resolve(ctx, use_cache).await })
	}
}

#[doc(hidden)]
pub trait __InjectDependsRegistryResolver<T>
where
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, Depends<T>>
	where
		T: 'a;
}

impl<T> __InjectDependsRegistryResolver<T> for &__InjectDependsResolver<T>
where
	T: Send + Sync + 'static,
{
	fn __resolve_inject_depends_parameter<'a>(
		self,
		ctx: &'a InjectionContext,
		use_cache: bool,
	) -> __InjectResolveFuture<'a, Depends<T>>
	where
		T: 'a,
	{
		Box::pin(async move { Depends::<T>::resolve_from_registry(ctx, use_cache).await })
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
			Depends::<T>::resolve(ctx, use_cache)
				.await
				.map(Depends::into_inner)
		})
	}
}
