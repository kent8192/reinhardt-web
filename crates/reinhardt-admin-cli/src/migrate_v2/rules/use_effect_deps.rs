//! Rule §6.1 (4): add an explicit dependency list to hooks that omit it.
//!
//! Tracked reads are migrated to `deps![...]` in source order. When the
//! callback expression cannot be identified, the rule retains the explicit
//! `compile_error!` review marker used by the original v2 migration.

use std::collections::HashSet;

use quote::quote;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `use_effect_deps` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"use_effect_deps"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		let mut visitor = HookVisitor::default();
		visitor.visit_file_mut(&mut file);
		if visitor.generated_deps && !file_imports_deps(&file) {
			let facade = hook_facade(&file).unwrap_or_else(|| syn::parse_quote!(reinhardt_pages));
			let import: syn::Item =
				syn::parse2(quote!(use #facade::deps;)).expect("generated deps import must parse");
			let insertion = file
				.items
				.iter()
				.position(|item| matches!(item, syn::Item::Use(_)))
				.unwrap_or(0);
			file.items.insert(insertion, import);
		}
		file
	}
}

#[derive(Default)]
struct HookVisitor {
	generated_deps: bool,
}

/// Hooks whose v2 signature requires an explicit dependency list as the final argument.
const HOOKS_REQUIRING_DEPS: &[&str] = &[
	"use_effect",
	"use_layout_effect",
	"use_retained_effect",
	"use_retained_layout_effect",
	"use_memo",
	"use_callback",
	"use_callback_with",
	"use_resource",
	"use_resource_with_key",
];

impl VisitMut for HookVisitor {
	fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
		if let syn::Expr::Path(path) = &*call.func
			&& let Some(segment) = path.path.segments.last()
			&& HOOKS_REQUIRING_DEPS.contains(&segment.ident.to_string().as_str())
		{
			if call.args.len() == 1 {
				let callback = call.args.first().expect("one callback argument must exist");
				let deps = dependency_list(callback);
				if deps.is_some() {
					self.generated_deps = true;
				}
				call.args.push(deps.unwrap_or_else(review_marker));
			} else if call.args.len() == 2
				&& let Some(deps) = tuple_dependency_list(
					call.args
						.last()
						.expect("two-argument hook must have dependencies"),
				) {
				self.generated_deps = true;
				*call
					.args
					.last_mut()
					.expect("two-argument hook must have dependencies") = deps;
			}
		}
		visit_mut::visit_expr_call_mut(self, call);
	}
}

fn tuple_dependency_list(deps: &syn::Expr) -> Option<syn::Expr> {
	let syn::Expr::Tuple(tuple) = deps else {
		return None;
	};
	let dependencies = tuple.elems.iter().map(strip_clone);
	Some(
		syn::parse2(quote!(deps![#(#dependencies),*]))
			.expect("generated dependency list must parse"),
	)
}

fn strip_clone(expr: &syn::Expr) -> &syn::Expr {
	if let syn::Expr::MethodCall(call) = expr
		&& call.method == "clone"
		&& call.args.is_empty()
	{
		&call.receiver
	} else {
		expr
	}
}

fn dependency_list(callback: &syn::Expr) -> Option<syn::Expr> {
	let mut reads = TrackedReadVisitor::default();
	reads.visit_expr(callback);
	if !reads.saw_closure {
		return None;
	}
	let captured = reads
		.reads
		.iter()
		.map(|name| syn::Ident::new(name, proc_macro2::Span::call_site()));
	Some(syn::parse2(quote!(deps![#(#captured),*])).expect("generated dependency list must parse"))
}

fn review_marker() -> syn::Expr {
	syn::parse_quote! {
		compile_error!(
			"manouche-v2 codemod: add explicit deps list here, e.g. `deps![count]`"
		)
	}
}

#[derive(Default)]
struct TrackedReadVisitor {
	reads: Vec<String>,
	seen: HashSet<String>,
	saw_closure: bool,
}

impl<'ast> Visit<'ast> for TrackedReadVisitor {
	fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
		self.saw_closure = true;
		visit::visit_expr_closure(self, closure);
	}

	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		let method = call.method.to_string();
		let tracked_read = (matches!(method.as_str(), "get" | "into_value")
			&& call.args.is_empty())
			|| (method == "with" && call.args.len() == 1);
		if tracked_read
			&& let Some(ident) = base_ident_of(&call.receiver)
			&& self.seen.insert(ident.to_string())
		{
			self.reads.push(ident.to_string());
		}
		visit::visit_expr_method_call(self, call);
	}
}

fn base_ident_of(expr: &syn::Expr) -> Option<&syn::Ident> {
	match expr {
		syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
			Some(&path.path.segments[0].ident)
		}
		syn::Expr::MethodCall(call) => base_ident_of(&call.receiver),
		syn::Expr::Field(field) => base_ident_of(&field.base),
		syn::Expr::Paren(paren) => base_ident_of(&paren.expr),
		syn::Expr::Reference(reference) => base_ident_of(&reference.expr),
		syn::Expr::Try(try_expr) => base_ident_of(&try_expr.expr),
		_ => None,
	}
}

fn file_imports_deps(file: &syn::File) -> bool {
	file.items.iter().any(
		|item| matches!(item, syn::Item::Use(item_use) if use_tree_contains(&item_use.tree, "deps")),
	)
}

fn hook_facade(file: &syn::File) -> Option<syn::Path> {
	file.items.iter().find_map(|item| {
		let syn::Item::Use(item_use) = item else {
			return None;
		};
		if !HOOKS_REQUIRING_DEPS
			.iter()
			.any(|hook| use_tree_contains(&item_use.tree, hook))
		{
			return None;
		}
		let syn::UseTree::Path(root) = &item_use.tree else {
			return None;
		};
		Some(syn::Path::from(root.ident.clone()))
	})
}

fn use_tree_contains(tree: &syn::UseTree, needle: &str) -> bool {
	match tree {
		syn::UseTree::Path(path) => path.ident == needle || use_tree_contains(&path.tree, needle),
		syn::UseTree::Name(name) => name.ident == needle,
		syn::UseTree::Rename(rename) => rename.ident == needle || rename.rename == needle,
		syn::UseTree::Glob(_) => false,
		syn::UseTree::Group(group) => group
			.items
			.iter()
			.any(|item| use_tree_contains(item, needle)),
	}
}
