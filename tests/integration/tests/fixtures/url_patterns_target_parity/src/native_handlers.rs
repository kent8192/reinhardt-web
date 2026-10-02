use reinhardt::{Response, ViewResult};

#[cfg(feature = "client-router")]
#[reinhardt::get("/native-pages/health/", name = "native-page-health", auth = "public")]
pub async fn page_health() -> ViewResult<Response> {
	Ok(Response::ok().with_body(native_only_fixture::NativeMarker::body()))
}

#[reinhardt::get("/health/", name = "health", auth = "public")]
pub async fn health() -> ViewResult<Response> {
	Ok(Response::ok().with_body(native_only_fixture::NativeMarker::body()))
}

#[reinhardt::post(
	"/protected/",
	name = "protected",
	auth = "protected",
	guard = "fixture credential gate"
)]
pub async fn protected() -> ViewResult<Response> {
	Ok(Response::ok().with_body("protected-handler"))
}
