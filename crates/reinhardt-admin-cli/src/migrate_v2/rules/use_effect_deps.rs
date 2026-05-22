//! Rule §6.1 (4): `use_effect(closure)` →
//! `use_effect(closure, compile_error!("add deps"))`.
//!
//! Per spec footnote, (4) is semi-automatic: the rule inserts a
//! `compile_error!` placeholder as the new deps argument so the human
//! must consciously add the dependency tuple. Auto-inferring deps would
//! lock in subtle bugs (forgotten closures, accidental captures).

use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `use_effect_deps` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"use_effect_deps"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		HookVisitor.visit_file_mut(&mut file);
		file
	}
}

struct HookVisitor;

/// Hooks whose v2 signature requires an explicit deps tuple as the final arg.
const HOOKS_REQUIRING_DEPS: &[&str] = &[
	"use_effect",
	"use_layout_effect",
	"use_memo",
	"use_callback",
	"use_callback_with",
];

impl VisitMut for HookVisitor {
	fn visit_expr_call_mut(&mut self, c: &mut syn::ExprCall) {
		// Match the bare path of the call.
		if let syn::Expr::Path(p) = &*c.func
			&& let Some(seg) = p.path.segments.last()
		{
			let name = seg.ident.to_string();
			if HOOKS_REQUIRING_DEPS.contains(&name.as_str()) && c.args.len() == 1 {
				let placeholder: syn::Expr = syn::parse_quote! {
					compile_error!(
						"manouche-v2 codemod: add explicit deps tuple here, e.g. `(count.clone(),)`"
					)
				};
				c.args.push(placeholder);
			}
		}
		visit_mut::visit_expr_call_mut(self, c);
	}
}
