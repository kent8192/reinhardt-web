//! Reinhardt REST Tutorial Example - Code Snippet Management API
//!
//! This example demonstrates the concepts covered in the Reinhardt REST tutorial:
//! - Serialization and validation
//! - Request and response handling
//! - Class-based views
//! - Authentication and permissions
//! - ViewSets and routers

// The `#[reinhardt::viewset]` attribute in `apps/snippets/views.rs` expands
// into a `macro_rules!` definition that is then referenced through the
// crate-absolute path `$crate::__for_each_viewset_*!` by the `#[routes]`
// macro. That composition trips the
// `macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompat
// lint (rust-lang/rust#52234) which is `deny`-by-default. The framework's
// own integration tests apply the same crate-level allow (see
// `tests/integration/tests/url_patterns_viewset_typed_integration.rs`) until
// the framework reworks the manifest macros to avoid absolute paths. Remove
// this allow once Phase 6.2's `__for_each_viewset_*!` indirection no longer
// goes through `$crate::`.
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]

pub mod apps;
pub mod config;
pub mod native_runtime;
