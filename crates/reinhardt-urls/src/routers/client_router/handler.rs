//! Route handler abstractions for typed parameter extraction.
//!
//! This module provides the [`RouteHandler`] trait and implementations
//! for different handler signatures, enabling type-safe path parameter
//! extraction in route definitions.

use super::error::RouterError;
use super::from_request::{FromRequest, RouteContext};
use super::params::{FromPath, ParamContext, Path, SingleFromPath};
use reinhardt_core::types::page::Page;
use std::marker::PhantomData;
use std::sync::Arc;

/// Trait for route handlers that can handle route requests.
///
/// This trait abstracts over different handler signatures:
/// - `Fn() -> Page` - No parameters
/// - `Fn(Path<T>) -> Page` - With typed parameters
/// - `Fn(Path<T>) -> Result<Page, E>` - With error handling
pub trait RouteHandler: Send + Sync {
	/// Handles the route request with the given parameter context.
	///
	/// # Errors
	///
	/// Returns [`RouterError::PathExtraction`] if parameter extraction fails.
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError>;
}

/// Handler for routes without parameters.
///
/// Wraps a `Fn() -> Page` closure.
pub(crate) struct NoParamsHandler<F> {
	handler: F,
}

impl<F> NoParamsHandler<F> {
	/// Creates a new no-params handler.
	pub(crate) fn new(handler: F) -> Self {
		Self { handler }
	}
}

// SAFETY: NoParamsHandler is Send + Sync when F is Send + Sync
unsafe impl<F: Send> Send for NoParamsHandler<F> {}
unsafe impl<F: Sync> Sync for NoParamsHandler<F> {}

impl<F> RouteHandler for NoParamsHandler<F>
where
	F: Fn() -> Page + Send + Sync,
{
	fn handle(&self, _ctx: &ParamContext) -> Result<Page, RouterError> {
		Ok((self.handler)())
	}
}

/// Handler for routes with typed parameters.
///
/// Wraps a `Fn(Path<T>) -> Page` closure.
pub(crate) struct WithParamsHandler<F, T> {
	handler: F,
	_phantom: PhantomData<T>,
}

impl<F, T> WithParamsHandler<F, T> {
	/// Creates a new with-params handler.
	pub(crate) fn new(handler: F) -> Self {
		Self {
			handler,
			_phantom: PhantomData,
		}
	}
}

// SAFETY: WithParamsHandler is Send + Sync when F is Send + Sync
// The PhantomData<T> is only used for type inference and doesn't hold data
unsafe impl<F: Send, T> Send for WithParamsHandler<F, T> {}
unsafe impl<F: Sync, T> Sync for WithParamsHandler<F, T> {}

impl<F, T> RouteHandler for WithParamsHandler<F, T>
where
	F: Fn(Path<T>) -> Page + Send + Sync,
	T: FromPath + Send + Sync,
{
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
		let params = Path::<T>::from_path(ctx).map_err(RouterError::PathExtraction)?;
		Ok((self.handler)(params))
	}
}

/// Handler for routes that return `Result<Page, E>`.
///
/// Wraps a `Fn(Path<T>) -> Result<Page, E>` closure.
pub(crate) struct ResultHandler<F, T, E> {
	handler: F,
	_phantom: PhantomData<(T, E)>,
}

impl<F, T, E> ResultHandler<F, T, E> {
	/// Creates a new result handler.
	pub(crate) fn new(handler: F) -> Self {
		Self {
			handler,
			_phantom: PhantomData,
		}
	}
}

// SAFETY: ResultHandler is Send + Sync when F is Send + Sync
// The PhantomData<(T, E)> is only used for type inference and doesn't hold data
unsafe impl<F: Send, T, E> Send for ResultHandler<F, T, E> {}
unsafe impl<F: Sync, T, E> Sync for ResultHandler<F, T, E> {}

impl<F, T, E> RouteHandler for ResultHandler<F, T, E>
where
	F: Fn(Path<T>) -> Result<Page, E> + Send + Sync,
	T: FromPath + Send + Sync,
	E: Into<RouterError> + Send + Sync,
{
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
		let params = Path::<T>::from_path(ctx).map_err(RouterError::PathExtraction)?;
		(self.handler)(params).map_err(|e| e.into())
	}
}

/// Helper function to create a no-params handler.
pub(crate) fn no_params_handler<F>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn() -> Page + Send + Sync + 'static,
{
	Arc::new(NoParamsHandler::new(handler))
}

/// Helper function to create a with-params handler.
pub(crate) fn with_params_handler<F, T>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn(Path<T>) -> Page + Send + Sync + 'static,
	T: FromPath + Send + Sync + 'static,
{
	Arc::new(WithParamsHandler::new(handler))
}

/// Helper function to create a result handler.
pub(crate) fn result_handler<F, T, E>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn(Path<T>) -> Result<Page, E> + Send + Sync + 'static,
	T: FromPath + Send + Sync + 'static,
	E: Into<RouterError> + Send + Sync + 'static,
{
	Arc::new(ResultHandler::new(handler))
}

// ============================================================================
// Arity-generic Handler<Args> trait (axum-style)
// ============================================================================
//
// Replaces the per-arity `SinglePathHandler` / `TwoPathHandler` /
// `ThreePathHandler` structs with a single `Handler<Args>` trait whose
// `Args` is inferred from the closure signature. `Args` is a marker tuple
// (e.g., `(T1,)`, `(T1, T2)`, …) so the compiler picks the right impl
// without forcing the caller to name an arity in the method.
//
// The trait is sealed: only the framework can add impls. This keeps the
// shape of `Args` free to grow (for example, to mix `Path<T>`, `Query<Q>`,
// and `State<S>` extractors) without it being a breaking change.

mod sealed {
	/// Sealed marker — only impls inside this crate can satisfy
	/// [`super::Handler`]. The `Args` type parameter mirrors
	/// `Handler<Args>` so that each per-arity impl reaches the tuple
	/// type from its trait parameter (Rust would otherwise reject the
	/// `T1..Tn` of the where-clause as unconstrained — E0207). Refs
	/// Issue #4637.
	pub trait Sealed<Args> {}
}

/// Sealed trait implemented for every closure shape `Fn(Path<T1>, …,
/// Path<Tn>) -> Page` for `n` in 1..=8.
///
/// `Args` is a marker tuple `(T1, …, Tn)` that the compiler infers from
/// the closure signature; callers never name it explicitly. To register
/// the closure as a [`RouteHandler`], call
/// [`Handler::into_route_handler`].
///
/// # Sealing
///
/// Users do **not** implement this trait — every legal impl lives in
/// the [`impl_handler!`] macro expansion below. Sealing the trait keeps
/// the shape of `Args` an internal concern so a future revision can mix
/// `Path<T>` with `Query<Q>` / `State<S>` extractors without breaking
/// downstream code.
///
/// Refs Issue #4637.
pub trait Handler<Args>: sealed::Sealed<Args> + Send + Sync + 'static {
	/// Convert this closure into a type-erased `Arc<dyn RouteHandler>`
	/// suitable for storage inside a `ClientRoute`.
	fn into_route_handler(self) -> Arc<dyn RouteHandler>;
}

/// Internal storage type that pairs a closure `F` with its inferred
/// marker tuple `Args`. The `Args` parameter is phantom; it exists only
/// so multiple `RouteHandler` impls (one per arity) can coexist on a
/// single struct without `Args` leaking Send/Sync constraints onto `F`.
pub(crate) struct PathHandler<F, Args> {
	handler: F,
	// `PhantomData<fn(Args) -> ()>` so `Args` is contravariant and does
	// not impose `Send + Sync` on the wrapper independently of `F`.
	_phantom: PhantomData<fn(Args) -> ()>,
}

// SAFETY: `Args` lives only in `PhantomData<fn(Args) -> ()>`, which is
// `Send + Sync` for every `Args`. Thread safety of `PathHandler` is
// therefore identical to thread safety of `F`.
unsafe impl<F: Send, Args> Send for PathHandler<F, Args> {}
unsafe impl<F: Sync, Args> Sync for PathHandler<F, Args> {}

/// Generate per-arity `Sealed` / `Handler<Args>` / `RouteHandler` impls
/// for a closure shape `Fn(Path<T1>, …, Path<Tn>) -> Page`.
///
/// Invoked once per arity below (`impl_handler!([T1], [0]);` for
/// `n = 1`, up through `[T1..T8]` / `[0..7]` for `n = 8`). Each
/// expansion is gated on a distinct `Fn` arity, so the resulting
/// `Handler<(T1, …, Tn)>` impls do not overlap (Rust treats `Fn`s of
/// different arities as distinct trait bounds — axum's `all_the_tuples!`
/// uses the same trick).
macro_rules! impl_handler {
	([$($Ti:ident),+], [$($idx:tt),+]) => {
		impl<F, $($Ti),+> sealed::Sealed<($($Ti,)+)> for F
		where
			F: Fn($(Path<$Ti>),+) -> Page + Send + Sync + 'static,
			$($Ti: SingleFromPath + Send + Sync + 'static,)+
		{}

		impl<F, $($Ti),+> Handler<($($Ti,)+)> for F
		where
			F: Fn($(Path<$Ti>),+) -> Page + Send + Sync + 'static,
			$($Ti: SingleFromPath + Send + Sync + 'static,)+
		{
			fn into_route_handler(self) -> Arc<dyn RouteHandler> {
				Arc::new(PathHandler::<F, ($($Ti,)+)> {
					handler: self,
					_phantom: PhantomData,
				})
			}
		}

		impl<F, $($Ti),+> RouteHandler for PathHandler<F, ($($Ti,)+)>
		where
			F: Fn($(Path<$Ti>),+) -> Page + Send + Sync,
			$($Ti: SingleFromPath + Send + Sync,)+
		{
			#[allow(non_snake_case)] // matches the macro's tuple type idents
			fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
				$(let $Ti = $Ti::from_path_at(ctx, $idx)
					.map_err(RouterError::PathExtraction)?;)+
				Ok((self.handler)($(Path($Ti)),+))
			}
		}
	};
}

impl_handler!([T1], [0]);
impl_handler!([T1, T2], [0, 1]);
impl_handler!([T1, T2, T3], [0, 1, 2]);
impl_handler!([T1, T2, T3, T4], [0, 1, 2, 3]);
impl_handler!([T1, T2, T3, T4, T5], [0, 1, 2, 3, 4]);
impl_handler!([T1, T2, T3, T4, T5, T6], [0, 1, 2, 3, 4, 5]);
impl_handler!([T1, T2, T3, T4, T5, T6, T7], [0, 1, 2, 3, 4, 5, 6]);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8], [0, 1, 2, 3, 4, 5, 6, 7]);

// ============================================================================
// FromRequest-based page handler (spec §4.3)
// ============================================================================

/// Handler for routes registered via [`ClientRouter::page`].
///
/// Wraps a `Fn(P) -> Page` closure where `P: FromRequest`. Constructs
/// `P` from the matched path / query data via `P::from_request`, then
/// invokes the closure. Extraction errors are surfaced as a
/// `Page::Text` describing the failure so client-side navigation does
/// not panic; a future iteration may route these through a dedicated
/// error page.
///
/// [`ClientRouter::page`]: super::core::ClientRouter::page
pub(crate) struct FromRequestHandler<F, P> {
	handler: F,
	pattern: String,
	_phantom: PhantomData<P>,
}

impl<F, P> FromRequestHandler<F, P> {
	pub(crate) fn new(handler: F, pattern: String) -> Self {
		Self {
			handler,
			pattern,
			_phantom: PhantomData,
		}
	}
}

// SAFETY: FromRequestHandler is Send + Sync when F is Send + Sync.
// The PhantomData<P> is only used for type inference and does not
// hold data — matching the established pattern in this file.
unsafe impl<F: Send, P> Send for FromRequestHandler<F, P> {}
unsafe impl<F: Sync, P> Sync for FromRequestHandler<F, P> {}

impl<F, P> RouteHandler for FromRequestHandler<F, P>
where
	F: Fn(P) -> Page + Send + Sync,
	P: FromRequest + Send + Sync,
{
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
		let route_ctx = RouteContext::new(
			// The matched path is not directly available to RouteHandler;
			// pass an empty placeholder. Handlers that need the path can
			// read it from the router's `current_path` signal.
			String::new(),
			ctx.params().clone(),
			ctx.query().unwrap_or("").to_string(),
		);
		match P::from_request(&route_ctx) {
			Ok(props) => Ok((self.handler)(props)),
			Err(e) => Ok(Page::Text(
				format!("route extraction error on `{}`: {e}", self.pattern).into(),
			)),
		}
	}
}

/// Helper function to create a `FromRequest`-based handler.
pub(crate) fn from_request_handler<F, P>(handler: F, pattern: String) -> Arc<dyn RouteHandler>
where
	F: Fn(P) -> Page + Send + Sync + 'static,
	P: FromRequest + Send + Sync + 'static,
{
	Arc::new(FromRequestHandler::new(handler, pattern))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::routers::client_router::error::PathError;
	use std::collections::HashMap;

	fn test_page() -> Page {
		Page::Empty
	}

	fn page_with_text(s: &str) -> Page {
		Page::Text(s.to_string().into())
	}

	#[test]
	fn test_no_params_handler() {
		let handler = NoParamsHandler::new(test_page);
		let ctx = ParamContext::new(HashMap::new(), Vec::new());

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_with_params_handler() {
		let handler =
			WithParamsHandler::new(|Path(id): Path<i32>| page_with_text(&format!("ID: {}", id)));

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_with_params_handler_error() {
		let handler = WithParamsHandler::new(|Path(_id): Path<i32>| Page::Empty);

		let ctx = ParamContext::new(HashMap::new(), vec!["not_a_number".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_err());

		match result {
			Err(RouterError::PathExtraction(PathError::ParseError { .. })) => {}
			_ => panic!("Expected PathExtraction error"),
		}
	}

	#[test]
	fn test_result_handler_ok() {
		let handler = ResultHandler::new(|Path(id): Path<i32>| {
			if id > 0 {
				Ok(page_with_text(&format!("ID: {}", id)))
			} else {
				Err(RouterError::NotFound("Invalid ID".to_string()))
			}
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_result_handler_err() {
		let handler = ResultHandler::new(|Path(id): Path<i32>| {
			if id > 0 {
				Ok(Page::Empty)
			} else {
				Err(RouterError::NotFound("Invalid ID".to_string()))
			}
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["-1".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_err());

		match result {
			Err(RouterError::NotFound(_)) => {}
			_ => panic!("Expected NotFound error"),
		}
	}

	#[test]
	fn test_with_params_handler_tuple() {
		let handler = WithParamsHandler::new(|Path((user_id, post_id)): Path<(i64, i64)>| {
			page_with_text(&format!("User: {}, Post: {}", user_id, post_id))
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["123".to_string(), "456".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_helper_no_params_handler() {
		let handler: Arc<dyn RouteHandler> = no_params_handler(test_page);
		let ctx = ParamContext::new(HashMap::new(), Vec::new());

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_helper_with_params_handler() {
		let handler: Arc<dyn RouteHandler> =
			with_params_handler(|Path(id): Path<i32>| page_with_text(&format!("ID: {}", id)));

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[test]
	fn test_helper_result_handler() {
		let handler: Arc<dyn RouteHandler> = result_handler(|Path(id): Path<i32>| {
			if id > 0 {
				Ok(page_with_text(&format!("ID: {}", id)))
			} else {
				Err(RouterError::NotFound("Invalid ID".to_string()))
			}
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}
}
