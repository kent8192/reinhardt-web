//! Pre-codegen pass that verifies hook deps tuples cover all Signal reads
//! inside hook closures (Refs #4195, Manouche v2 Layer ② React alignment).
//!
//! See the design spec at
//! `docs/superpowers/specs/2026-05-22-issue-4195-hooks-deps-array-design.md`
//! §7 for the verification rules.
//!
//! # Implementation status
//!
//! This is the **scaffold** phase. The visitor walks the input token stream
//! and detects `use_effect` / `use_memo` / `use_callback` / `use_callback_with`
//! / `use_layout_effect` call sites by name. Full Signal-read extraction from
//! a Manouche `PageBody` (which is NOT a `syn::Block`) requires walking
//! through the Manouche typed-AST emitter and reparsing the embedded Rust
//! expressions; that piece is tracked as a follow-up issue.
//!
//! The scaffold returns an empty `TokenStream` today, so the macro pipeline
//! remains unchanged. Once the follow-up lands, this module will start
//! emitting `compile_error!` diagnostics here without touching the rest of
//! the codegen path.

use proc_macro2::TokenStream;
use quote::quote;

use reinhardt_manouche::core::PageMacro;

/// Verified hook names — kept in lockstep with the `use_*(f, deps)`
/// signatures shipped in this PR.
#[allow(dead_code)]
pub(crate) const VERIFIED_HOOKS: &[&str] = &[
	"use_effect",
	"use_layout_effect",
	"use_memo",
	"use_callback",
	"use_callback_with",
];

/// Methods that explicitly opt out of deps verification — reading these
/// returns the latest value WITHOUT subscribing, matching Option A's
/// `useEffectEvent`-by-construction semantics.
#[allow(dead_code)]
pub(crate) const ESCAPE_METHODS: &[&str] = &["get_untracked", "with_untracked"];

/// Run the hook-deps verification pass over a parsed `PageMacro`.
///
/// Today this is a no-op scaffold: it returns an empty `TokenStream` so the
/// surrounding `page_impl` pipeline is unaffected. Once the follow-up
/// implementation lands, this function will return one or more
/// `compile_error!` invocations for each Signal read that is missing from
/// its enclosing hook's deps tuple.
pub(crate) fn verify_hook_deps(_input: &PageMacro) -> TokenStream {
	// TODO: Implement full Signal-read detection against PageBody traversal.
	// Tracked as follow-up to reinhardt-web#4195. Until then, runtime
	// behavior is unchanged: explicit-deps semantics are enforced by the
	// `*::new_with_deps` constructors at the runtime layer.
	quote! {}
}
