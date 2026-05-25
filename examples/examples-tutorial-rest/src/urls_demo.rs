//! Typed URL helpers backed by `ResolvedUrls`.
//!
//! This module is the canonical demonstration of the **typed
//! `ResolvedUrls` accessor pattern** introduced by Issue #4507 / PR #4518.
//! Instead of formatting URLs inline (`format!("/api/snippets/{id}/")`)
//! every helper below delegates to the typed gateway
//! `urls.server().<app>().<route>()` so that the URL strings stay in lock-
//! step with the `#[get]` / `#[post]` / `#[viewset]` route definitions in
//! `apps::snippets::views`.
//!
//! The deprecated flat surface (`urls.snippets_list()` etc.) that
//! preceded the typed pattern was removed in 0.2.0 per Issue #4520.
//!
//! ## Why typed accessors
//!
//! 1. **Compile-time discoverability** — every route registered with
//!    `name = "snippets_list"` etc. surfaces as a typed method on
//!    `SnippetsUrls<'_>`. Misspelling a route name is a compile error, not
//!    a `panic!("Route not found")` at request time.
//! 2. **Namespace safety** — the per-app gateway
//!    (`urls.server().snippets()`) auto-prefixes route names with the
//!    `"snippets:"` namespace. The deprecated flat surface relies on a
//!    runtime namespace-iteration fallback (see `UrlResolverUnprefixed`
//!    in `reinhardt-urls`).
//! 3. **Refactor-safe** — when a route path changes in
//!    `apps::snippets::urls`, every call site that goes through this
//!    module continues to resolve to the new path automatically.
//!
//! ## Usage
//!
//! Callers obtain a `ResolvedUrls` once per request (via `from_global()`
//! after the server has registered routes) and call any helper:
//!
//! ```rust,ignore
//! use examples_tutorial_rest::urls_demo;
//! use reinhardt::ResolvedUrls;
//!
//! let urls = ResolvedUrls::from_global();
//! let list_url   = urls_demo::snippets_list(&urls);          // "/api/snippets/"
//! let detail_url = urls_demo::snippets_retrieve(&urls, 42);  // "/api/snippets/42/"
//! let vs_list    = urls_demo::viewset_list(&urls);           // "/api/snippets-viewset/"
//! ```
//!
//! For a fully worked end-to-end test that registers routes and exercises
//! every helper below, see `tests/urls_typed_accessors.rs`.

use crate::config::urls::ResolvedUrls;

// ----------------------------------------------------------------------------
// Function-based endpoints (Tutorial 1-5)
//
// Routes registered via `#[get] / #[post] / #[put] / #[delete]` in
// `apps::snippets::views` with `name = "snippets_<verb>"`. The route names
// become methods on the `SnippetsUrls<'_>` accessor returned by
// `urls.server().snippets()` (the route name suffix `_list`, `_create`,
// etc. is preserved verbatim — only the `"snippets:"` namespace is added
// transparently).
// ----------------------------------------------------------------------------

/// Resolve `GET /api/snippets/` (list endpoint).
pub fn snippets_list(urls: &ResolvedUrls) -> String {
	urls.server().snippets().snippets_list()
}

/// Resolve `POST /api/snippets/` (create endpoint).
pub fn snippets_create(urls: &ResolvedUrls) -> String {
	urls.server().snippets().snippets_create()
}

/// Resolve `GET /api/snippets/{id}/` (retrieve endpoint).
///
/// The typed accessor takes the path parameter as a `&str`. Convert any
/// non-string primary key with `to_string()` at the call site — keeping
/// the typed parameter signature `&str` avoids leaking ORM-specific types
/// into the URL surface.
pub fn snippets_retrieve(urls: &ResolvedUrls, id: i64) -> String {
	urls.server().snippets().snippets_retrieve(&id.to_string())
}

/// Resolve `PUT /api/snippets/{id}/` (update endpoint).
pub fn snippets_update(urls: &ResolvedUrls, id: i64) -> String {
	urls.server().snippets().snippets_update(&id.to_string())
}

/// Resolve `DELETE /api/snippets/{id}/` (delete endpoint).
pub fn snippets_delete(urls: &ResolvedUrls, id: i64) -> String {
	urls.server().snippets().snippets_delete(&id.to_string())
}

// ----------------------------------------------------------------------------
// ViewSet endpoints (Tutorial 6)
//
// Registered via `.viewset("/snippets-viewset", views::viewset())` in
// `apps::snippets::urls` against a `ModelViewSet::new("snippet")`. The
// viewset basename `"snippet"` (singular — chosen to make the typed names
// idiomatic) drives the generated route names: `snippet_list`,
// `snippet_detail`, `snippet_create`, `snippet_update`, `snippet_partial_update`,
// `snippet_destroy`. Compare with the function-based endpoints above whose
// names start with the plural `snippets_` because they were registered
// individually by `#[get(name = "snippets_list")]` etc.
// ----------------------------------------------------------------------------

/// Resolve `GET /api/snippets-viewset/` (viewset list endpoint).
pub fn viewset_list(urls: &ResolvedUrls) -> String {
	urls.server().snippets().snippet_list()
}

/// Resolve `GET /api/snippets-viewset/{id}/` (viewset retrieve endpoint).
pub fn viewset_detail(urls: &ResolvedUrls, id: i64) -> String {
	urls.server().snippets().snippet_detail(&id.to_string())
}
