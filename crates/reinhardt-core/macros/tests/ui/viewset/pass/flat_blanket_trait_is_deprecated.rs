//! Verifies that the impl-form `#[viewset(basename = "...")]` introduced for
//! Issue #4507 expands cleanly on a bare struct (no `#[action]` methods).
//!
//! This is the minimum self-contained shape that exercises the new
//! impl-form API surface. The fn-form variant
//! (`#[viewset] pub fn viewset() -> ModelViewSet<...>`) cannot be tested
//! here because the generated resolver modules reference
//! `reinhardt_core::UrlResolverUnprefixed`, and pulling
//! `reinhardt-core` into the `reinhardt-macros` dev-dep tree would create
//! a circular dependency (see `tests/ui.rs` header comment).
//!
//! The actual deprecation warning on the flat blanket-trait surface is
//! exercised at runtime in
//! `tests/integration/tests/url_patterns_viewset_typed_integration.rs`.
//!
//! Refs Issue #4507.

// `reinhardt_macros::viewset` is brought in only so the `#[viewset]`
// attribute expansion below has its proc-macro driver; the name is not
// referenced from any path expression. Permit the otherwise-unused import.
#![allow(unused_imports)]
// `SnippetViewSet` exists only to give `#[viewset]` something to attach to;
// it is never constructed or referenced. Permit the dead struct.
#![allow(dead_code)]

use reinhardt_macros::viewset;

pub struct SnippetViewSet;

#[viewset(basename = "snippet")]
impl SnippetViewSet {}

fn main() {}
