
//! Client-side router for the Tier 4 fixture.
//!
//! Registers four **named** routes whose names follow the
//! `namespace:identifier` convention used by Reinhardt Cloud — see
//! [`init_router`] below for the exact names (`auth:login`,
//! `dashboard:home`, `clusters:list`, `deployments:list`). Tier 4
//! exists specifically to give Inv-5 (`history.state.route_name ==
//! matched route.name()`) a code path with a non-empty `name()` to
//! read.
//!
//! [`init_router`] is invoked once by `super::lib::main` through
//! `ClientLauncher::router_client`. From any component, call [`with_spa_router`]
//! (re-exported from `reinhardt-pages`) to inspect routing state at
//! render time.

use reinhardt_urls::routers::ClientRouter;

pub use reinhardt_pages::app::with_spa_router;

use super::pages::{clusters_page, deployments_page, home_page, login_page};

/// Build the Tier 4 application router.
///
/// All four routes are registered with `named_route` so that
/// `route.name()` returns `Some(...)` and `Router::navigate` writes
/// the namespaced name into `history.state.route_name`. Anonymous
/// `route(...)` registrations would leave that field empty and miss
/// the regression class Tier 4 is meant to catch.
pub fn init_router() -> ClientRouter {
	ClientRouter::new()
		.named_route("dashboard:home", "/", home_page)
		.named_route("clusters:list", "/clusters", clusters_page)
		.named_route("deployments:list", "/deployments", deployments_page)
		.named_route("auth:login", "/login", login_page)
}
