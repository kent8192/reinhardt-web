//! Reinhardt REST Tutorial Example - Code Snippet Management API
//!
//! This example demonstrates the concepts covered in the Reinhardt REST tutorial:
//! - Serialization and validation
//! - Request and response handling
//! - Class-based views
//! - Authentication and permissions
//! - ViewSets and routers
//! - Typed `ResolvedUrls` accessors (see [`urls_demo`])

// The `#[reinhardt::viewset]` attribute in `apps/snippets/views.rs` expands
// into a `macro_rules!` definition that is then referenced through the
// crate-absolute path `$crate::__for_each_viewset_*!` by sibling macros
// (`#[url_patterns]`, `#[routes]`). That composition trips the
// `macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompat
// lint (rust-lang/rust#52234) which is `deny`-by-default. The framework's
// own integration tests apply the same crate-level allow (see
// `tests/integration/tests/url_patterns_viewset_typed_integration.rs`) until
// the framework reworks the manifest macros to avoid absolute paths. Remove
// this allow once Phase 6.2's `__for_each_viewset_*!` indirection no longer
// goes through `$crate::`.
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
// The `#[reinhardt::viewset]` macro also expands into a blanket impl of the
// deprecated `UrlResolverUnprefixed` trait (which powers the flat
// `urls.snippet_list()` / `urls.snippet_detail("id")` accessors, deprecated
// since `0.1.0-rc.16`). The deprecation warning fires at the macro
// attribute's call site, which is module-level — outside any function body
// where a local `#[allow(deprecated)]` would apply. The typed accessor
// surface demonstrated in `src/urls_demo.rs` and exercised in
// `tests/urls_typed_accessors.rs` is the migration target. Remove this
// crate-level allow once the framework drops the flat trait emission
// (planned alongside the `0.2.0` flat-surface removal — see Issue #4548
// § "Deprecation removal milestone").
#![allow(deprecated)]

pub mod apps;
pub mod config;
pub mod urls_demo;
