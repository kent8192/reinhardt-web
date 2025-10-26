//! URL routing and proxy utilities for Reinhardt framework.
//!
//! This crate provides URL routing, pattern matching, and proxy functionality
//! for the Reinhardt web framework. It is a unified interface over the following
//! internal crates:
//!
//! - `reinhardt-routers`: URL routing and pattern matching
//! - `reinhardt-routers-macros`: Compile-time URL validation macros
//! - `reinhardt-proxy`: Lazy relationship loading for ORM
//!
//! ## Planned Features
//! TODO: Add route middleware support

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "routers")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers")))]
pub use reinhardt_routers as routers;

#[cfg(feature = "routers-macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers-macros")))]
pub use reinhardt_routers_macros as routers_macros;

#[cfg(feature = "proxy")]
#[cfg_attr(docsrs, doc(cfg(feature = "proxy")))]
pub use reinhardt_proxy as proxy;

// Re-export commonly used types from routers
#[cfg(feature = "routers")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers")))]
pub mod prelude {
    pub use reinhardt_routers::{
        PathPattern, Route, Router, clear_script_prefix, get_script_prefix, set_script_prefix,
    };
}
