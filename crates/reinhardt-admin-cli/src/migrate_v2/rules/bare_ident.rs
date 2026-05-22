//! Rule §6.1 (1): `tag { ident }` → `tag { {ident} }` (no-op stub; implemented in Task 3).

use crate::migrate_v2::rewriter::FileRewriter;

/// Placeholder rule — replaced by the real implementation in Task 3.
pub struct Rule;

impl FileRewriter for Rule {
	fn rewrite(&self, file: syn::File) -> syn::File {
		file
	}
	fn name(&self) -> &'static str {
		"bare_ident"
	}
}
