//! Route handler abstractions for typed parameter extraction.
//!
//! This module provides the [`RouteHandler`] trait and implementations
//! for different handler signatures, enabling type-safe path parameter
//! extraction in route definitions.

use super::core::RouterError;
use super::params::{FromPath, ParamContext, PathParams};
use crate::component::Page;
use std::marker::PhantomData;
use std::sync::Arc;

/// Trait for route handlers that can handle route requests.
///
/// This trait abstracts over different handler signatures:
/// - `Fn() -> Page` - No parameters
/// - `Fn(PathParams<T>) -> Page` - With typed parameters
/// - `Fn(PathParams<T>) -> Result<Page, E>` - With error handling
pub(super) trait RouteHandler: Send + Sync {
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
pub(super) struct NoParamsHandler<F> {
	handler: F,
}

impl<F> NoParamsHandler<F> {
	/// Creates a new no-params handler.
	pub(super) fn new(handler: F) -> Self {
		Self { handler }
	}
}

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
/// Wraps a `Fn(PathParams<T>) -> Page` closure.
pub(super) struct WithParamsHandler<F, T> {
	handler: F,
	_phantom: PhantomData<T>,
}

impl<F, T> WithParamsHandler<F, T> {
	/// Creates a new with-params handler.
	pub(super) fn new(handler: F) -> Self {
		Self {
			handler,
			_phantom: PhantomData,
		}
	}
}

impl<F, T> RouteHandler for WithParamsHandler<F, T>
where
	F: Fn(PathParams<T>) -> Page + Send + Sync,
	T: FromPath + Send + Sync,
{
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
		let params = PathParams::<T>::from_path(ctx).map_err(RouterError::PathExtraction)?;
		Ok((self.handler)(params))
	}
}

/// Handler for routes that return `Result<Page, E>`.
///
/// Wraps a `Fn(PathParams<T>) -> Result<Page, E>` closure.
pub(super) struct ResultHandler<F, T, E> {
	handler: F,
	_phantom: PhantomData<(T, E)>,
}

impl<F, T, E> ResultHandler<F, T, E> {
	/// Creates a new result handler.
	pub(super) fn new(handler: F) -> Self {
		Self {
			handler,
			_phantom: PhantomData,
		}
	}
}

impl<F, T, E> RouteHandler for ResultHandler<F, T, E>
where
	F: Fn(PathParams<T>) -> Result<Page, E> + Send + Sync,
	T: FromPath + Send + Sync,
	E: Into<RouterError> + Send + Sync,
{
	fn handle(&self, ctx: &ParamContext) -> Result<Page, RouterError> {
		let params = PathParams::<T>::from_path(ctx).map_err(RouterError::PathExtraction)?;
		(self.handler)(params).map_err(|e| e.into())
	}
}

/// Helper function to create a no-params handler.
pub(super) fn no_params_handler<F>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn() -> Page + Send + Sync + 'static,
{
	Arc::new(NoParamsHandler::new(handler))
}

/// Helper function to create a with-params handler.
pub(super) fn with_params_handler<F, T>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn(PathParams<T>) -> Page + Send + Sync + 'static,
	T: FromPath + Send + Sync + 'static,
{
	Arc::new(WithParamsHandler::new(handler))
}

/// Helper function to create a result handler.
pub(super) fn result_handler<F, T, E>(handler: F) -> Arc<dyn RouteHandler>
where
	F: Fn(PathParams<T>) -> Result<Page, E> + Send + Sync + 'static,
	T: FromPath + Send + Sync + 'static,
	E: Into<RouterError> + Send + Sync + 'static,
{
	Arc::new(ResultHandler::new(handler))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::router::PathError;
	use rstest::rstest;
	use std::collections::HashMap;

	fn test_view() -> Page {
		Page::text("Test")
	}

	#[rstest]
	fn test_no_params_handler() {
		let handler = NoParamsHandler::new(test_view);
		let ctx = ParamContext::new(HashMap::new(), Vec::new());

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_with_params_handler() {
		let handler = WithParamsHandler::new(|PathParams(id): PathParams<i32>| {
			Page::text(format!("ID: {}", id))
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());

		let view = result.unwrap();
		assert_eq!(view.render_to_string(), "ID: 42");
	}

	#[rstest]
	fn test_with_params_handler_error() {
		let handler = WithParamsHandler::new(|PathParams(id): PathParams<i32>| {
			Page::text(format!("ID: {}", id))
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["not_a_number".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_err());

		match result {
			Err(RouterError::PathExtraction(PathError::ParseError { .. })) => {}
			_ => panic!("Expected PathExtraction error"),
		}
	}

	#[rstest]
	fn test_result_handler_ok() {
		let handler = ResultHandler::new(|PathParams(id): PathParams<i32>| {
			if id > 0 {
				Ok(Page::text(format!("ID: {}", id)))
			} else {
				Err(RouterError::NotFound("Invalid ID".to_string()))
			}
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_result_handler_err() {
		let handler = ResultHandler::new(|PathParams(id): PathParams<i32>| {
			if id > 0 {
				Ok(Page::text(format!("ID: {}", id)))
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

	#[rstest]
	fn test_with_params_handler_tuple() {
		let handler =
			WithParamsHandler::new(|PathParams((user_id, post_id)): PathParams<(i64, i64)>| {
				Page::text(format!("User: {}, Post: {}", user_id, post_id))
			});

		let ctx = ParamContext::new(HashMap::new(), vec!["123".to_string(), "456".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());

		let view = result.unwrap();
		assert_eq!(view.render_to_string(), "User: 123, Post: 456");
	}

	#[rstest]
	fn test_helper_no_params_handler() {
		let handler = no_params_handler(test_view);
		let ctx = ParamContext::new(HashMap::new(), Vec::new());

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_helper_with_params_handler() {
		let handler = with_params_handler(|PathParams(id): PathParams<i32>| {
			Page::text(format!("ID: {}", id))
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_helper_result_handler() {
		let handler = result_handler(|PathParams(id): PathParams<i32>| {
			if id > 0 {
				Ok(Page::text(format!("ID: {}", id)))
			} else {
				Err(RouterError::NotFound("Invalid ID".to_string()))
			}
		});

		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = handler.handle(&ctx);
		assert!(result.is_ok());
	}
}
