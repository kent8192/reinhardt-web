//! Verifies that `#[viewset]` on an `impl` block without `basename = "..."`
//! produces a clear error message pointing the user at the correct syntax.
//!
//! Refs Issue #4507.

use reinhardt_macros::viewset;

pub struct SnippetViewSet;

#[viewset]
impl SnippetViewSet {}

fn main() {}
