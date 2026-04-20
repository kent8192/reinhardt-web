//! Views module for {{ app_name }} app (RESTful)
// Add your view submodules here. Each `pub mod` declaration
// corresponds to a file under the `views/` directory.
//
// For multi-file views that need re-exports for discovery, use:
// flatten_imports! {
//     pub mod example;
// }
//
// Example of an authenticated endpoint using `AuthUser<U>` (rc.15+):
// `AuthUser<U>` resolves the authenticated user via DI — JWT verification
// is handled automatically by the auth middleware.
//
// use crate::models::User; // replace with your user model
// use reinhardt::{get, use_inject, AuthUser, Response, StatusCode};
// use reinhardt::http::ViewResult;
//
// #[use_inject]
// #[get("/me/", name = "{{ app_name }}_me")]
// pub async fn me(
//     #[inject] AuthUser(user): AuthUser<User>,
// ) -> ViewResult<Response> {
//     Ok(Response::new(StatusCode::OK).with_body(user.email().to_string()))
// }
