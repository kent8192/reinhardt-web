//! Rewriter trait composed across rules.

/// One mechanical migration step in the Manouche v1 → v2 codemod.
///
/// Each rule receives the parsed AST of a single `.rs` file and returns a
/// (possibly) transformed copy. Rules are composed sequentially by the
/// driver in `migrate_v2::run`.
pub trait FileRewriter {
	/// Returns a (possibly) transformed copy of the input file AST.
	fn rewrite(&self, file: syn::File) -> syn::File;

	/// Short name for `--skip` filtering and reporting.
	fn name(&self) -> &'static str;
}
