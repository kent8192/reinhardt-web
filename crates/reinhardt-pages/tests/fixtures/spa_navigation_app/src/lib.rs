//! Minimal SPA fixture used by `spa_navigation_e2e_test` to verify
//! that `<a href="/...">` clicks trigger SPA navigation and route
//! re-rendering against a real Chrome browser via CDP.
//!
//! Two routes:
//! - `/` renders `<div id="route-home"><a href="/login">Go to login</a></div>`
//! - `/login` renders `<div id="route-login">LOGIN VIEW</div>`
//!
//! Refs #4088.

mod client;

pub use client::start;
