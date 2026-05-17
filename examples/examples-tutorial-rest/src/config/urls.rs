//! URL configuration for examples-tutorial-rest project
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! The `/api/` prefix is a literal path (no `{...}` parameters), which
//! satisfies the rc.24 guard that panics if `ServerRouter::mount()` receives
//! a prefix containing path parameters.
//!
//! Plain `#[routes]` (no flags) is used instead of `#[routes(standalone)]`
//! because this project consumes `installed_apps!` (see
//! `src/config/apps.rs`). The non-standalone form generates the
//! `crate::urls::url_prelude` module and per-app `ResolvedUrls::<app>()`
//! accessors that aggregate the `url_resolvers` modules emitted by
//! `#[url_patterns(InstalledApp::snippets, mode = server)]` in
//! `src/apps/snippets/urls.rs`. The `standalone` flag would suppress
//! both, which is only appropriate for projects without `installed_apps!`.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
