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
// `#[get]` auto-enables DI when `#[inject]` parameters are present.
//
// use crate::models::User; // replace with your user model
// use reinhardt::{get, AuthUser, Response, StatusCode};
// use reinhardt::http::ViewResult;
//
// #[get("/me/", name = "{{ app_name }}_me")]
// pub async fn me(
//     #[inject] AuthUser(user): AuthUser<User>,
// ) -> ViewResult<Response> {
//     Ok(Response::new(StatusCode::OK).with_body(user.email().to_string()))
// }
//
// Generic DI for app services uses `Depends<T>` (rc.16+) — the replacement
// for the deprecated `Arc<T>` / `Injected<T>` parameter form:
//
// use reinhardt::extract::Depends;
//
// #[get("/health/", name = "{{ app_name }}_health")]
// pub async fn health(
//     #[inject] svc: Depends<MyService>,
// ) -> ViewResult<Response> {
//     Ok(Response::new(StatusCode::OK).with_body(svc.status()))
// }
//
// For declarative endpoint-level authorization, use `guard!()` (rc.16+):
//
// use reinhardt::guard;
//
// #[get("/admin/", name = "{{ app_name }}_admin", guards = guard!(IsStaff))]
// pub async fn admin_only() -> ViewResult<Response> { /* ... */ }
