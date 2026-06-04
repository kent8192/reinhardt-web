//! Organized re-exports of the reinhardt facade API.

mod macros;
pub use macros::*;

#[cfg(all(feature = "conf", native))]
mod settings;
#[cfg(all(feature = "conf", native))]
pub use settings::*;

#[cfg(native)]
mod core_types;
#[cfg(native)]
pub use core_types::*;

#[cfg(all(feature = "database", native))]
mod database;
#[cfg(all(feature = "database", native))]
pub use database::*;

#[cfg(all(feature = "auth", native))]
mod auth;
#[cfg(all(feature = "auth", native))]
pub use auth::*;

#[cfg(all(feature = "rest", native))]
mod rest;
#[cfg(all(feature = "rest", native))]
pub use rest::*;

#[cfg(all(any(feature = "standard", feature = "middleware"), native))]
mod middleware_exports;
#[cfg(all(any(feature = "standard", feature = "middleware"), native))]
pub use middleware_exports::*;

#[cfg(feature = "routing")]
mod routing;
#[cfg(feature = "routing")]
pub use routing::*;

#[cfg(all(
	any(feature = "api", feature = "standard", feature = "api-only"),
	native
))]
mod views;
#[cfg(all(
	any(feature = "api", feature = "standard", feature = "api-only"),
	native
))]
pub use views::*;

#[cfg(all(feature = "forms", native))]
mod forms;
#[cfg(all(feature = "forms", native))]
pub use forms::*;

#[cfg(all(any(feature = "minimal", feature = "standard", feature = "di"), native))]
mod di;
#[cfg(all(any(feature = "minimal", feature = "standard", feature = "di"), native))]
pub use di::*;

#[cfg(native)]
mod misc;
#[cfg(native)]
pub use misc::*;

// Disambiguate names that appear in multiple export modules via glob.
// `reinhardt_rest::openapi::*` re-exports utoipa's `Response` and `Header`
// which shadow the framework's own types. Explicit re-exports take
// precedence over globs, restoring the original crate-root behavior.
#[cfg(all(feature = "core", native))]
pub use core_types::Response;
#[cfg(all(any(feature = "minimal", feature = "standard", feature = "di"), native))]
pub use di::Header;
