//! Rule §6.1 (4): inject explicit deps placeholder for `use_effect`-family hooks
//! (no-op stub; implemented in Task 5).

use crate::migrate_v2::rewriter::FileRewriter;

/// Placeholder rule — replaced by the real implementation in Task 5.
pub struct Rule;

impl FileRewriter for Rule {
	fn rewrite(&self, file: syn::File) -> syn::File {
		file
	}
	fn name(&self) -> &'static str {
		"use_effect_deps"
	}
}
