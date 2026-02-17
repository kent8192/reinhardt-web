//! Dependency injection support for gRPC handlers
//!
//! This module provides extensions to integrate Reinhardt's DI system with gRPC handlers.
//!
//! # Overview
//!
//! The DI system for gRPC works by storing an `InjectionContext` in the request extensions,
//! which can then be extracted and used to resolve dependencies.
//!
//! # Example
//!
//! ```rust,no_run,ignore
//! # use reinhardt_grpc::{GrpcRequestExt, grpc_handler};
//! # use reinhardt_di::InjectionContext;
//! # use tonic::{Request, Response, Status};
//! # use std::sync::Arc;
//! # struct GetUserRequest { id: i64 }
//! # struct User;
//! # struct DatabaseConnection;
//! # impl DatabaseConnection {
//! #     async fn fetch_user(&self, _id: i64) -> Result<User, Status> { Ok(User) }
//! # }
//! # #[tonic::async_trait]
//! # trait UserService {
//! #     async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status>;
//! # }
//! pub struct UserServiceImpl {
//!     injection_context: Arc<InjectionContext>,
//! }
//!
//! #[tonic::async_trait]
//! impl UserService for UserServiceImpl {
//!     async fn get_user(&self, mut request: Request<GetUserRequest>)
//!         -> Result<Response<User>, Status>
//!     {
//!         request.extensions_mut().insert(self.injection_context.clone());
//!         self.get_user_impl(request).await
//!     }
//! }
//!
//! impl UserServiceImpl {
//!     #[grpc_handler]
//!     async fn get_user_impl(
//!         &self,
//!         request: Request<GetUserRequest>,
//!         #[inject] db: DatabaseConnection,
//!     ) -> Result<Response<User>, Status> {
//!         let user_id = request.into_inner().id;
//!         let user = db.fetch_user(user_id).await?;
//!         Ok(Response::new(user))
//!     }
//! }
//! ```

/// Extension trait for `tonic::Request` to support DI context extraction
///
/// This trait adds methods to `tonic::Request` for working with Reinhardt's
/// dependency injection system.
pub trait GrpcRequestExt {
	/// Extract DI context from request extensions
	///
	/// Returns `Some(T)` if the context exists, `None` otherwise.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// # use reinhardt_grpc::GrpcRequestExt;
	/// # use reinhardt_di::InjectionContext;
	/// # use std::sync::Arc;
	/// # use tonic::Request;
	/// # fn example<T>(request: &Request<T>) -> Option<Arc<InjectionContext>> {
	/// let di_ctx = request.get_di_context::<Arc<InjectionContext>>()?;
	/// # Some(di_ctx)
	/// # }
	/// ```
	fn get_di_context<T: Clone + Send + Sync + 'static>(&self) -> Option<T>;
}

impl<T> GrpcRequestExt for tonic::Request<T> {
	fn get_di_context<C: Clone + Send + Sync + 'static>(&self) -> Option<C> {
		self.extensions().get::<C>().cloned()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_di::InjectionContext;
	use rstest::rstest;
	use std::sync::Arc;
	use tonic::Request;

	#[rstest]
	fn test_grpc_request_ext_get_di_context() {
		// Create a mock InjectionContext
		let singleton_scope = reinhardt_di::SingletonScope::new();
		let injection_ctx = Arc::new(InjectionContext::builder(singleton_scope).build());

		// Create a request and insert the context
		let mut request = Request::new(());
		request.extensions_mut().insert(injection_ctx.clone());

		// Extract the context using the extension trait
		let extracted = request
			.get_di_context::<Arc<InjectionContext>>()
			.expect("DI context should exist");

		// Verify it's the same context
		assert!(Arc::ptr_eq(&injection_ctx, &extracted));
	}

	#[rstest]
	fn test_grpc_request_ext_missing_context() {
		// Create a request without DI context
		let request = Request::new(());

		// Try to extract the context
		let extracted = request.get_di_context::<Arc<InjectionContext>>();

		// Should be None
		assert!(extracted.is_none());
	}
}
