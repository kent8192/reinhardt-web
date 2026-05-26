//! Tier 4 fixture — SPA navigation regression suite (#4203).
//!
//! Mirrors Tier 3 (`spa_navigation_with_full_layout_app`) — persistent
//! `<aside>` sidebar + per-route content swap inside `<main>` — but uses
//! **named** routes with namespace-style names matching the Reinhardt
//! Cloud convention (`auth:login`, `dashboard:home`, etc. — see the
//! Routes table below for the exact names this fixture registers).
//!
//! Tier 1〜3 use simple route names; Tier 4 uses namespace-style names.
//! `Router::navigate` writes `route_match.route.name().unwrap_or("")`
//! into `history.state.route_name`. Tier 4
//! exists specifically to exercise namespaced names and make Inv-5 (history
//! `route_name` == matched named-route name) and Inv-6 (`__diag_router_id`
//! invariant across the click/render path) testable.
//!
//! Routes:
//! - `/`            named `dashboard:home`
//! - `/clusters`    named `clusters:list`
//! - `/deployments` named `deployments:list`
//! - `/login`       named `auth:login`
//!
//! This crate is the WASM-only slice of a `reinhardt-admin startproject
//! --with-pages` scaffold; the server-side modules (`apps`, `config`,
//! `server_fn`, `shared`, `bin/manage.rs`, `settings/`) were dropped
//! because the fixture is driven directly by `wasm-pack` and the e2e
//! harness, not by `manage runserver`.

// Client-only modules (WASM). The `cfg(client)` alias is defined in
// `build.rs` (`target_arch = "wasm32"`).
#[cfg(client)]
pub mod client;
