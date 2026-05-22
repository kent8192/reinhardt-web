//! Rule §6.2: migrate `#[derive(Default)] struct *Props` to `#[derive(bon::Builder)]`
//! (no-op stub; implemented in Task 6).

use crate::migrate_v2::rewriter::FileRewriter;

/// Placeholder rule — replaced by the real implementation in Task 6.
pub struct Rule;

impl FileRewriter for Rule {
	fn rewrite(&self, file: syn::File) -> syn::File {
		file
	}
	fn name(&self) -> &'static str {
		"component_props"
	}
}
