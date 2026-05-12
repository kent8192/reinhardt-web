//! Proc-macro re-exports for `reinhardt`.
//!
//! Procedural macros are host-side and target-agnostic, so most entries here
//! are cross-target. A handful (`api_view`/`delete`/`get`/...) are only
//! re-exported on native because the runtime types they reference are server-
//! only. The existing per-item `#[cfg]` gates are preserved verbatim from the
//! pre-refactor `src/lib.rs` so the public API surface is unchanged.

// Issue #4161: `AppConfig` (derive), `app_config` (attribute), and `installed_apps`
// are proc-macros that run host-side; the macro-emitted code references
// `::reinhardt::macros::AppConfig` and `::reinhardt::reinhardt_apps::*`.
// Re-exporting them on wasm (matching #4156's pattern for routes/url_patterns)
// enables downstream client crates to use `#[app_config]` and `#[url_patterns]`
// cross-target. The actual runtime types they reference are provided by the
// wasm shim modules in `crate::compat`.
pub use reinhardt_macros::{AppConfig, app_config, installed_apps};

// Re-export settings attribute macro (requires conf feature)
#[cfg(all(feature = "conf", native))]
pub use reinhardt_macros::settings;

// Re-export Model derive macro and model attribute macro (requires database feature)
#[cfg(all(feature = "database", native))]
pub use reinhardt_macros::{Model, model};

// Re-export collect_migrations macro (requires database feature)
#[cfg(all(feature = "database", native))]
pub use reinhardt_macros::collect_migrations;

// The `::reinhardt::macros::*` namespace used by macro-generated code lives
// at the crate root (see `src/lib.rs`), not here, to avoid `module_inception`
// (a `pub mod macros` inside `exports/macros.rs` would nest the same name).

// Re-export HTTP method macros (native only — runtime types are server-side)
#[cfg(native)]
pub use reinhardt_macros::{api_view, delete, get, patch, post, put};

// Re-export `flatten_imports` (native only)
#[cfg(native)]
pub use reinhardt_macros::flatten_imports;

// `routes` and `url_patterns` are cross-target per Issue #4156.
pub use reinhardt_macros::{routes, url_patterns};

// `viewset` macro (native only)
#[cfg(native)]
pub use reinhardt_macros::viewset;

// Re-export admin attribute macro (requires admin feature)
#[cfg(all(feature = "admin", native))]
pub use reinhardt_macros::admin;

// Re-export ApplyUpdate derive + apply_update attribute macro (native only)
#[cfg(native)]
pub use reinhardt_macros::{ApplyUpdate as DeriveApplyUpdate, apply_update};
