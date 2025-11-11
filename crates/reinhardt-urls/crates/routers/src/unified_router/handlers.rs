//! Handler adapters for ViewSets and functions

use async_trait::async_trait;
use reinhardt_core::{
	Handler,
	http::{Request, Response, Result},
};
// use reinhardt_core::di::InjectionContext; // TODO: Enable when ViewSet supports DI
use reinhardt_viewsets::{Action, ViewSet};
use std::sync::Arc;

/// Handler adapter for ViewSets
pub(crate) struct ViewSetHandler {
	pub viewset: Arc<dyn ViewSet>,
	pub action: Action,
}

#[async_trait]
impl Handler for ViewSetHandler {
	async fn handle(&self, req: Request) -> Result<Response> {
		// TODO: DI support - check if ViewSet supports DI
		// if self.viewset.supports_di()
		// 	&& let Some(di_ctx) = req.get_di_context::<InjectionContext>()
		// {
		// 	// Use DI-aware dispatch
		// 	return self
		// 		.viewset
		// 		.dispatch_with_context(req, self.action, &di_ctx)
		// 		.await;
		// }

		// Regular dispatch
		self.viewset.dispatch(req, self.action).await
	}
}

/// Function handler adapter
pub struct FunctionHandler<F> {
	pub func: F,
}

#[async_trait]
impl<F, Fut> Handler for FunctionHandler<F>
where
	F: Fn(Request) -> Fut + Send + Sync,
	Fut: std::future::Future<Output = Result<Response>> + Send,
{
	async fn handle(&self, req: Request) -> Result<Response> {
		(self.func)(req).await
	}
}
