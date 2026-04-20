//! Views module for {{ app_name }} app (RESTful)
// Add your view submodules here. Each `pub mod` declaration
// corresponds to a file under the `views/` directory.
//
// For multi-file views that need re-exports for discovery, use:
// flatten_imports! {
//     pub mod example;
// }
//
// Example of a JWT-protected endpoint using typed `JwtError` (rc.15+):
//
// use reinhardt::{get, JwtAuth, JwtError, Response, StatusCode};
// use reinhardt::http::ViewResult;
// use axum::extract::Query;
// use serde::Deserialize;
//
// #[derive(Deserialize)]
// struct TokenQuery {
//     token: String,
// }
//
// #[get("/protected/", name = "{{ app_name }}_protected")]
// pub async fn protected(
//     Query(params): Query<TokenQuery>,
// ) -> ViewResult<Response> {
//     let jwt = JwtAuth::new(b"your_secret"); // load from settings in practice
//     match jwt.verify_token(&params.token) {
//         Ok(claims) => Ok(Response::new(StatusCode::OK).with_body(claims.username)),
//         Err(JwtError::TokenExpired) => {
//             Ok(Response::new(StatusCode::UNAUTHORIZED).with_body("Token expired"))
//         }
//         Err(JwtError::InvalidSignature(_)) => {
//             Ok(Response::new(StatusCode::UNAUTHORIZED).with_body("Invalid signature"))
//         }
//         Err(e) => Ok(Response::new(StatusCode::INTERNAL_SERVER_ERROR).with_body(e.to_string())),
//     }
// }
