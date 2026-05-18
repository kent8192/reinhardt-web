//! Verifies that the explicit `basename = "..."` form of `#[viewset]`
//! introduced for Issue #4549 expands cleanly on an `impl` block and
//! does NOT emit a deprecation warning.
//!
//! The impl-form is used here (rather than the fn-form) because the
//! fn-form expansion references `reinhardt_core::UrlResolverUnprefixed`
//! and pulling `reinhardt-core` into the `reinhardt-macros` dev-dep
//! tree would create a circular dependency (see `tests/ui.rs` header
//! comment). The deprecation flow itself is exercised by unit tests
//! in `viewset_macro.rs::tests`.
//!
//! Refs Issue #4549.

// `reinhardt_macros::viewset` is referenced only via the attribute
// expansion below.
#![allow(unused_imports)]
// `SnippetsViewSet` exists only to give `#[viewset]` something to
// attach to; it is never constructed.
#![allow(dead_code)]
// Treat the absence of any deprecation warning as part of the
// contract: explicit `basename = "..."` callers must compile cleanly
// even under the strict lint setting that the fallback case would
// fail under.
#![deny(deprecated)]

use reinhardt_macros::viewset;

pub struct SnippetsViewSet;

#[viewset(basename = "snippets")]
impl SnippetsViewSet {}

fn main() {}
