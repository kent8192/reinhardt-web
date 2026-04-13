use reinhardt::http::ViewResult;
use reinhardt::{Response, StatusCode, get};
use reinhardt::core::serde::json;

#[get("/", name = "root")]
pub async fn root() -> ViewResult<Response> {
    let body = json::json!({"message": "Rough2Spec API", "version": "0.1.0"});
    Ok(Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/json")
        .with_body(json::to_vec(&body)?))
}
