//! Rule §6.1 (2)+(3): `watch { body }` and `#reactive { body }` unwrap
//! (no-op stub; implemented in Task 4).

use crate::migrate_v2::rewriter::FileRewriter;

/// Placeholder rule — replaced by the real implementation in Task 4.
pub struct Rule;

impl FileRewriter for Rule {
	fn rewrite(&self, file: syn::File) -> syn::File {
		file
	}
	fn name(&self) -> &'static str {
		"watch_unwrap"
	}
}
