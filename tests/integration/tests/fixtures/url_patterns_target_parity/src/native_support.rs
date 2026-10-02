use crate::evaluation::record;
use async_trait::async_trait;
use reinhardt::reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt::{Handler, Middleware, Request, Response, ServerRouter, ViewResult};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct Config(&'static str);

#[async_trait]
impl Injectable for Config {
	async fn inject(context: &InjectionContext) -> DiResult<Self> {
		Ok(context
			.singleton_scope()
			.get::<Self>()
			.expect("fixture singleton is registered")
			.as_ref()
			.clone())
	}
}

struct CredentialGate;

#[async_trait]
impl Middleware for CredentialGate {
	async fn process(
		&self,
		request: Request,
		next: Arc<dyn Handler>,
	) -> reinhardt::Result<Response> {
		if request.get_header("x-fixture-credential").as_deref() != Some("accepted") {
			record("denied");
			return Ok(Response::forbidden().with_body("denied"));
		}
		record("middleware-before");
		let response = next.handle(request).await?;
		record("middleware-after");
		Ok(response)
	}
}

#[reinhardt::get(
	"/secured/",
	name = "secured",
	auth = "protected",
	guard = "fixture credential gate"
)]
pub(crate) async fn secured(#[inject] config: Config) -> ViewResult<Response> {
	record("handler");
	Ok(Response::ok().with_body(config.0))
}

pub(crate) fn configure(server: ServerRouter) -> ServerRouter {
	let scope = Arc::new(SingletonScope::new());
	scope.set(Config("injected-configuration"));
	let context = Arc::new(InjectionContext::builder(scope).build());
	server
		.with_di_context(context)
		.with_middleware(CredentialGate)
		.endpoint(secured)
}
