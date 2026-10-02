//! Rule §6.1 (4): add an explicit dependency list to hooks that omit it.
//!
//! Tracked reads are migrated to `deps![...]` in source order. When the
//! callback expression cannot be identified, the rule retains the explicit
//! `compile_error!` review marker used by the original v2 migration.

use std::collections::{HashMap, HashSet};

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
		rewrite_scope(&mut file.items);
		file
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HookSpec {
	name: &'static str,
	callback_index: usize,
	deps_index: usize,
	omitted_arity: usize,
	explicit_arity: usize,
}

const HOOK_SPECS: &[HookSpec] = &[
	HookSpec {
		name: "use_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_layout_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_retained_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_retained_layout_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_memo",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_callback",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_callback_with",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_resource",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_resource_with_key",
		callback_index: 1,
		deps_index: 2,
		omitted_arity: 2,
		explicit_arity: 3,
	},
];

#[derive(Clone)]
struct UseEntry {
	original: Vec<syn::Ident>,
	bound: syn::Ident,
	attrs: Vec<syn::Attribute>,
}

#[derive(Clone)]
struct GeneratedDepsImport {
	facade: syn::Path,
	attrs: Vec<syn::Attribute>,
}

struct HookResolution {
	spec: HookSpec,
	imports: Option<Vec<GeneratedDepsImport>>,
}

struct HookVisitor {
	generated_imports: Vec<GeneratedDepsImport>,
	imports: Vec<UseEntry>,
	local_modules: HashSet<String>,
}

fn rewrite_scope(items: &mut Vec<syn::Item>) {
	let imports = collect_imports(items);
	let local_modules = items
		.iter()
		.filter_map(|item| match item {
			syn::Item::Mod(module) => Some(module.ident.to_string()),
			_ => None,
		})
		.collect();
	let mut visitor = HookVisitor {
		generated_imports: Vec::new(),
		imports: imports.clone(),
		local_modules,
	};
	for item in items.iter_mut() {
		visitor.visit_item_mut(item);
	}
	if !visitor.generated_imports.is_empty() {
		let insertion = items
			.iter()
			.position(|item| matches!(item, syn::Item::Use(_)))
			.unwrap_or(0);
		let unconditional_facades = visitor
			.generated_imports
			.iter()
			.filter(|generated| generated.attrs.is_empty())
			.map(|generated| facade_key(&generated.facade))
			.collect::<HashSet<_>>();
		let mut inserted = 0;
		for generated in visitor.generated_imports {
			if !generated.attrs.is_empty()
				&& unconditional_facades.contains(&facade_key(&generated.facade))
			{
				continue;
			}
			if has_deps_import(&imports, &generated.facade, &generated.attrs) {
				continue;
			}
			let attrs = generated.attrs;
			let facade = generated.facade;
			let import = syn::parse2(quote!(#(#attrs)* use #facade::deps;))
				.expect("generated deps import must parse");
			items.insert(insertion + inserted, import);
			inserted += 1;
		}
	}
	for item in items.iter_mut() {
		if let syn::Item::Mod(module) = item
			&& let Some((_, nested)) = &mut module.content
		{
			rewrite_scope(nested);
		}
	}
}

impl VisitMut for HookVisitor {
	fn visit_item_mod_mut(&mut self, _module: &mut syn::ItemMod) {}

	fn visit_block_mut(&mut self, block: &mut syn::Block) {
		let previous_imports = self.imports.clone();
		let mut local_imports = Vec::new();
		for stmt in &block.stmts {
			if let syn::Stmt::Item(syn::Item::Use(item_use)) = stmt {
				flatten_use_tree(&item_use.tree, &[], &item_use.attrs, &mut local_imports);
			}
		}
		for local in local_imports {
			self.imports.retain(|entry| entry.bound != local.bound);
			self.imports.push(local);
		}
		visit_mut::visit_block_mut(self, block);
		self.imports = previous_imports;
	}

	fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
		if let Some(resolution) = self.resolve_hook(call) {
			let spec = resolution.spec;
			if call.args.len() == spec.omitted_arity {
				let callback = call
					.args
					.iter()
					.nth(spec.callback_index)
					.expect("hook callback must exist");
				let deps = resolution
					.imports
					.as_ref()
					.and_then(|_| dependency_list(callback));
				if deps.is_some()
					&& let Some(imports) = &resolution.imports
				{
					for import in imports {
						self.record_generated_import(import.facade.clone(), import.attrs.clone());
					}
				}
				call.args
					.insert(spec.deps_index, deps.unwrap_or_else(review_marker));
			} else if call.args.len() == spec.explicit_arity
				&& let Some(deps) = tuple_dependency_list(
					call.args
						.iter()
						.nth(spec.deps_index)
						.expect("hook dependencies must exist"),
				) {
				if let Some(imports) = &resolution.imports {
					for import in imports {
						self.record_generated_import(import.facade.clone(), import.attrs.clone());
					}
					*call
						.args
						.iter_mut()
						.nth(spec.deps_index)
						.expect("hook dependencies must exist") = deps;
				} else {
					*call
						.args
						.iter_mut()
						.nth(spec.deps_index)
						.expect("hook dependencies must exist") = review_marker();
				}
			}
		}
		visit_mut::visit_expr_call_mut(self, call);
	}
}

impl HookVisitor {
	fn record_generated_import(&mut self, facade: syn::Path, attrs: Vec<syn::Attribute>) {
		let key = import_key(&facade, &attrs);
		if self
			.generated_imports
			.iter()
			.all(|generated| import_key(&generated.facade, &generated.attrs) != key)
		{
			self.generated_imports
				.push(GeneratedDepsImport { facade, attrs });
		}
	}

	fn resolve_hook(&self, call: &syn::ExprCall) -> Option<HookResolution> {
		let syn::Expr::Path(path) = &*call.func else {
			return None;
		};
		let segments: Vec<_> = path
			.path
			.segments
			.iter()
			.map(|segment| segment.ident.clone())
			.collect();
		let called = segments.last()?;
		if segments.len() == 1 {
			let candidates = self
				.imports
				.iter()
				.filter(|entry| entry.bound == *called)
				.filter_map(|entry| {
					let original = entry.original.last()?;
					let spec = hook_spec(&original.to_string())?;
					let import = facade_from_segments(&entry.original)
						.filter(|_| !self.is_local_facade(&entry.original))
						.map(|facade| GeneratedDepsImport {
							facade,
							attrs: entry.attrs.clone(),
						});
					Some((spec, import))
				})
				.collect::<Vec<_>>();
			if let Some((spec, _)) = candidates.first() {
				let spec = *spec;
				if candidates.iter().any(|(candidate, _)| *candidate != spec) {
					return Some(HookResolution {
						spec: select_hook_spec(&candidates, call.args.len()),
						imports: None,
					});
				}
				let matching = candidates
					.into_iter()
					.filter(|(candidate, _)| *candidate == spec)
					.collect::<Vec<_>>();
				let imports = matching
					.iter()
					.map(|(_, import)| import.clone())
					.collect::<Option<Vec<_>>>();
				return Some(HookResolution { spec, imports });
			}
			hook_spec(&called.to_string())?;
			return None;
		}
		let mut candidates = Vec::new();
		for (index, segment) in segments.iter().enumerate().take(segments.len() - 1) {
			for entry in self.imports.iter().filter(|entry| entry.bound == *segment) {
				let mut resolved = entry.original.clone();
				resolved.extend(segments.iter().skip(index + 1).cloned());
				let Some(original) = resolved.last() else {
					continue;
				};
				let Some(spec) = hook_spec(&original.to_string()) else {
					continue;
				};
				let import = facade_from_segments(&resolved)
					.filter(|_| !self.is_local_facade(&resolved))
					.map(|facade| GeneratedDepsImport {
						facade,
						attrs: entry.attrs.clone(),
					});
				candidates.push((spec, import));
			}
		}
		if let Some((spec, _)) = candidates.first() {
			let spec = *spec;
			if candidates.iter().any(|(candidate, _)| *candidate != spec) {
				return Some(HookResolution {
					spec: select_hook_spec(&candidates, call.args.len()),
					imports: None,
				});
			}
			let matching = candidates
				.into_iter()
				.filter(|(candidate, _)| *candidate == spec)
				.collect::<Vec<_>>();
			let imports = matching
				.iter()
				.map(|(_, import)| import.clone())
				.collect::<Option<Vec<_>>>();
			return Some(HookResolution { spec, imports });
		}
		let spec = hook_spec(&called.to_string())?;
		let facade = facade_from_segments(&segments).filter(|_| !self.is_local_facade(&segments));
		Some(HookResolution {
			spec,
			imports: facade.map(|facade| {
				vec![GeneratedDepsImport {
					facade,
					attrs: Vec::new(),
				}]
			}),
		})
	}

	fn is_local_facade(&self, segments: &[syn::Ident]) -> bool {
		let Some(first) = segments.first() else {
			return false;
		};
		if first == "crate" {
			return segments
				.iter()
				.any(|segment| self.local_modules.contains(&segment.to_string()));
		}
		self.local_modules.contains(&first.to_string())
	}
}

fn hook_spec(name: &str) -> Option<HookSpec> {
	HOOK_SPECS.iter().copied().find(|spec| spec.name == name)
}

fn select_hook_spec(
	candidates: &[(HookSpec, Option<GeneratedDepsImport>)],
	arity: usize,
) -> HookSpec {
	candidates
		.iter()
		.find(|(spec, _)| spec.omitted_arity == arity || spec.explicit_arity == arity)
		.map(|(spec, _)| *spec)
		.unwrap_or(candidates[0].0)
}

fn facade_from_segments(segments: &[syn::Ident]) -> Option<syn::Path> {
	if segments.is_empty() {
		return None;
	}
	if segments
		.first()
		.is_some_and(|ident| ident == "self" || ident == "super")
	{
		return None;
	}
	let end = segments
		.iter()
		.position(|ident| ident == "reactive")
		.unwrap_or(segments.len() - 1);
	if end == 0 {
		return None;
	}
	let facade = &segments[..end];
	syn::parse2(quote!(#(#facade)::* )).ok()
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
	let callback = strip_parens(callback);
	let (closure, aliases) = match callback {
		syn::Expr::Closure(closure) => (closure, HashMap::new()),
		syn::Expr::Block(block) => {
			let closure = block.block.stmts.last().and_then(stmt_closure)?;
			(
				closure,
				clone_aliases(&block.block.stmts[..block.block.stmts.len() - 1]),
			)
		}
		_ => return None,
	};
	let mut reads = OuterTrackedReadVisitor::default();
	for input in &closure.inputs {
		collect_pattern_idents(input, &mut reads.shadowed);
	}
	reads.visit_expr(&closure.body);
	if reads.nested_tracked_read || reads.shadowed_tracked_read {
		return None;
	}
	let mut dependencies = Vec::new();
	let mut seen = HashSet::new();
	for read in reads.reads {
		let dependency = resolve_alias(&read, &aliases)?;
		let key = quote!(#dependency).to_string();
		if seen.insert(key) {
			dependencies.push(dependency);
		}
	}
	Some(
		syn::parse2(quote!(deps![#(#dependencies),*]))
			.expect("generated dependency list must parse"),
	)
}

fn review_marker() -> syn::Expr {
	syn::parse_quote! {
		compile_error!(
			"manouche-v2 codemod: add explicit deps list here, e.g. `deps![count]`"
		)
	}
}

#[derive(Default)]
struct OuterTrackedReadVisitor {
	reads: Vec<String>,
	seen: HashSet<String>,
	shadowed: HashSet<String>,
	nested_tracked_read: bool,
	shadowed_tracked_read: bool,
}

impl<'ast> Visit<'ast> for OuterTrackedReadVisitor {
	fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
		let mut nested = AnyTrackedReadVisitor::default();
		nested.visit_expr_closure(closure);
		self.nested_tracked_read |= nested.found;
	}

	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		let method = call.method.to_string();
		let tracked_read = (matches!(method.as_str(), "get" | "into_value")
			&& call.args.is_empty())
			|| (method == "with" && call.args.len() == 1);
		if tracked_read && let Some(ident) = base_ident_of(&call.receiver) {
			let ident = ident.to_string();
			if self.shadowed.contains(&ident) {
				self.shadowed_tracked_read = true;
			} else if self.seen.insert(ident.clone()) {
				self.reads.push(ident);
			}
		}
		visit::visit_expr_method_call(self, call);
	}

	fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
		collect_pattern_idents(&expr.pat, &mut self.shadowed);
		visit::visit_expr_for_loop(self, expr);
	}

	fn visit_arm(&mut self, arm: &'ast syn::Arm) {
		collect_pattern_idents(&arm.pat, &mut self.shadowed);
		visit::visit_arm(self, arm);
	}

	fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
		collect_pattern_idents(&expr.pat, &mut self.shadowed);
		visit::visit_expr_let(self, expr);
	}

	fn visit_local(&mut self, local: &'ast syn::Local) {
		collect_pattern_idents(&local.pat, &mut self.shadowed);
		visit::visit_local(self, local);
	}
}

#[derive(Default)]
struct AnyTrackedReadVisitor {
	found: bool,
}

impl<'ast> Visit<'ast> for AnyTrackedReadVisitor {
	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		if is_tracked_read(call) {
			self.found = true;
		}
		visit::visit_expr_method_call(self, call);
	}
}

fn is_tracked_read(call: &syn::ExprMethodCall) -> bool {
	let method = call.method.to_string();
	(matches!(method.as_str(), "get" | "into_value") && call.args.is_empty())
		|| (method == "with" && call.args.len() == 1)
}

fn strip_parens(mut expr: &syn::Expr) -> &syn::Expr {
	while let syn::Expr::Paren(paren) = expr {
		expr = &paren.expr;
	}
	expr
}

fn stmt_closure(stmt: &syn::Stmt) -> Option<&syn::ExprClosure> {
	let syn::Stmt::Expr(expr, _) = stmt else {
		return None;
	};
	let syn::Expr::Closure(closure) = strip_parens(expr) else {
		return None;
	};
	Some(closure)
}

fn clone_aliases(stmts: &[syn::Stmt]) -> HashMap<String, syn::Expr> {
	let mut aliases = HashMap::new();
	for stmt in stmts {
		let syn::Stmt::Local(local) = stmt else {
			continue;
		};
		let mut bindings = HashSet::new();
		collect_pattern_idents(&local.pat, &mut bindings);
		let syn::Pat::Ident(alias) = &local.pat else {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			for binding in bindings {
				aliases.remove(&binding);
			}
			continue;
		};
		let alias_name = alias.ident.to_string();
		let Some(init) = &local.init else {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.remove(&alias_name);
			continue;
		};
		let syn::Expr::MethodCall(call) = strip_parens(&init.expr) else {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.remove(&alias_name);
			continue;
		};
		if call.method != "clone" || !call.args.is_empty() {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.remove(&alias_name);
			continue;
		}
		let Some(receiver) = safe_clone_place(&call.receiver) else {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.remove(&alias_name);
			continue;
		};
		let receiver_was_self = base_ident_of(&receiver).is_some_and(|ident| ident == &alias.ident);
		let receiver = resolve_place(&receiver, &aliases, &mut Vec::new());
		let receiver = receiver.filter(|resolved| {
			base_ident_of(resolved).is_none_or(|ident| ident != &alias.ident) || receiver_was_self
		});
		if let Some(receiver) = receiver {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.insert(alias_name, receiver);
		} else {
			invalidate_shadowed_aliases(&mut aliases, &bindings);
			aliases.remove(&alias.ident.to_string());
		}
	}
	aliases
}

fn invalidate_shadowed_aliases(
	aliases: &mut HashMap<String, syn::Expr>,
	bindings: &HashSet<String>,
) {
	let stale = aliases
		.iter()
		.filter_map(|(name, expr)| {
			base_ident_of(expr)
				.is_some_and(|base| bindings.contains(&base.to_string()))
				.then_some(name.clone())
		})
		.collect::<Vec<_>>();
	for name in stale {
		aliases.remove(&name);
	}
}

fn safe_clone_place(expr: &syn::Expr) -> Option<syn::Expr> {
	match strip_parens(expr) {
		syn::Expr::Path(path) if path.qself.is_none() && !path.path.segments.is_empty() => {
			Some(syn::Expr::Path(path.clone()))
		}
		syn::Expr::Field(field) => {
			let mut field = field.clone();
			field.base = Box::new(safe_clone_place(&field.base)?);
			Some(syn::Expr::Field(field))
		}
		_ => None,
	}
}

fn resolve_alias(name: &str, aliases: &HashMap<String, syn::Expr>) -> Option<syn::Expr> {
	let mut stack = vec![name.to_owned()];
	let expr = aliases.get(name)?;
	resolve_alias_place(expr, aliases, &mut stack)
}

fn resolve_alias_place(
	expr: &syn::Expr,
	aliases: &HashMap<String, syn::Expr>,
	stack: &mut Vec<String>,
) -> Option<syn::Expr> {
	match strip_parens(expr) {
		syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
			let ident = path.path.segments.first()?.ident.to_string();
			let Some(alias) = aliases.get(&ident) else {
				return Some(syn::Expr::Path(path.clone()));
			};
			if stack.last().is_some_and(|current| current == &ident) {
				return Some(syn::Expr::Path(path.clone()));
			}
			if stack.iter().any(|current| current == &ident) {
				return None;
			}
			stack.push(ident);
			let resolved = resolve_alias_place(alias, aliases, stack);
			stack.pop();
			resolved
		}
		_ => Some(strip_parens(expr).clone()),
	}
}

fn resolve_place(
	expr: &syn::Expr,
	aliases: &HashMap<String, syn::Expr>,
	stack: &mut Vec<String>,
) -> Option<syn::Expr> {
	match strip_parens(expr) {
		syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
			let ident = path.path.segments.first()?.ident.to_string();
			let Some(alias) = aliases.get(&ident) else {
				return Some(syn::Expr::Path(path.clone()));
			};
			if stack.last().is_some_and(|current| current == &ident) {
				return Some(syn::Expr::Path(path.clone()));
			}
			if stack.iter().any(|current| current == &ident) {
				return None;
			}
			stack.push(ident);
			let resolved = resolve_place(alias, aliases, stack);
			stack.pop();
			resolved
		}
		syn::Expr::Path(path) if path.qself.is_none() && !path.path.segments.is_empty() => {
			Some(syn::Expr::Path(path.clone()))
		}
		syn::Expr::Field(field) => {
			let mut field = field.clone();
			field.base = Box::new(resolve_place(&field.base, aliases, stack)?);
			Some(syn::Expr::Field(field))
		}
		_ => None,
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

fn collect_pattern_idents(pattern: &syn::Pat, bindings: &mut HashSet<String>) {
	match pattern {
		syn::Pat::Ident(ident) => {
			bindings.insert(ident.ident.to_string());
			if let Some((_, subpat)) = &ident.subpat {
				collect_pattern_idents(subpat, bindings);
			}
		}
		syn::Pat::Tuple(tuple) => {
			for element in &tuple.elems {
				collect_pattern_idents(element, bindings);
			}
		}
		syn::Pat::TupleStruct(tuple_struct) => {
			for element in &tuple_struct.elems {
				collect_pattern_idents(element, bindings);
			}
		}
		syn::Pat::Struct(struct_pattern) => {
			for field in &struct_pattern.fields {
				collect_pattern_idents(&field.pat, bindings);
			}
		}
		syn::Pat::Reference(reference) => collect_pattern_idents(&reference.pat, bindings),
		syn::Pat::Type(typed) => collect_pattern_idents(&typed.pat, bindings),
		syn::Pat::Or(or_pattern) => {
			for case in &or_pattern.cases {
				collect_pattern_idents(case, bindings);
			}
		}
		syn::Pat::Slice(slice) => {
			for element in &slice.elems {
				collect_pattern_idents(element, bindings);
			}
		}
		syn::Pat::Paren(paren) => collect_pattern_idents(&paren.pat, bindings),
		_ => {}
	}
}

fn collect_imports(items: &[syn::Item]) -> Vec<UseEntry> {
	let mut imports = Vec::new();
	for item in items {
		if let syn::Item::Use(item_use) = item {
			flatten_use_tree(&item_use.tree, &[], &item_use.attrs, &mut imports);
		}
	}
	imports
}

fn flatten_use_tree(
	tree: &syn::UseTree,
	prefix: &[syn::Ident],
	attrs: &[syn::Attribute],
	imports: &mut Vec<UseEntry>,
) {
	match tree {
		syn::UseTree::Path(path) => {
			let mut next = prefix.to_vec();
			next.push(path.ident.clone());
			flatten_use_tree(&path.tree, &next, attrs, imports);
		}
		syn::UseTree::Name(name) => {
			let mut original = prefix.to_vec();
			original.push(name.ident.clone());
			imports.push(UseEntry {
				original,
				bound: name.ident.clone(),
				attrs: attrs.to_vec(),
			});
		}
		syn::UseTree::Rename(rename) => {
			let mut original = prefix.to_vec();
			original.push(rename.ident.clone());
			imports.push(UseEntry {
				original,
				bound: rename.rename.clone(),
				attrs: attrs.to_vec(),
			});
		}
		syn::UseTree::Group(group) => {
			for tree in &group.items {
				flatten_use_tree(tree, prefix, attrs, imports);
			}
		}
		syn::UseTree::Glob(_) => {}
	}
}

fn import_key(facade: &syn::Path, attrs: &[syn::Attribute]) -> String {
	quote!(#facade #(#attrs)*).to_string()
}

fn facade_key(facade: &syn::Path) -> String {
	quote!(#facade).to_string()
}

fn has_deps_import(imports: &[UseEntry], facade: &syn::Path, attrs: &[syn::Attribute]) -> bool {
	let key = import_key(facade, attrs);
	let facade_path_key = facade_key(facade);
	imports.iter().any(|entry| {
		entry.bound == "deps"
			&& entry.original.last().is_some_and(|ident| ident == "deps")
			&& facade_from_segments(&entry.original).is_some_and(|entry_facade| {
				facade_key(&entry_facade) == facade_path_key
					&& (entry.attrs.is_empty() || import_key(&entry_facade, &entry.attrs) == key)
			})
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rewrite(source: &str) -> String {
		let file = syn::parse_file(source).expect("test source must parse");
		prettyplease::unparse(&Rule.rewrite(file))
	}

	fn compact(source: &str) -> String {
		source.chars().filter(|ch| !ch.is_whitespace()).collect()
	}

	#[test]
	fn hook_metadata_covers_all_supported_arities() {
		let actual: Vec<_> = HOOK_SPECS
			.iter()
			.map(|spec| {
				(
					spec.name,
					spec.callback_index,
					spec.deps_index,
					spec.omitted_arity,
					spec.explicit_arity,
				)
			})
			.collect();
		assert_eq!(
			actual,
			vec![
				("use_effect", 0, 1, 1, 2),
				("use_layout_effect", 0, 1, 1, 2),
				("use_retained_effect", 0, 1, 1, 2),
				("use_retained_layout_effect", 0, 1, 1, 2),
				("use_memo", 0, 1, 1, 2),
				("use_callback", 0, 1, 1, 2),
				("use_callback_with", 0, 1, 1, 2),
				("use_resource", 0, 1, 1, 2),
				("use_resource_with_key", 1, 2, 2, 3),
			]
		);
	}

	#[test]
	fn keyed_resource_adds_or_converts_third_dependency_argument() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::use_resource_with_key;
fn view(signal: Signal<i32>) {
    let _ = use_resource_with_key("a", || async { Ok::<_, ()>(1) });
    let _ = use_resource_with_key("b", || async { Ok::<_, ()>(2) }, ());
    let _ = use_resource_with_key("c", || async { Ok::<_, ()>(3) }, (signal.clone(),));
}
"#,
		));
		assert!(output.contains("use_resource_with_key(\"a\",||async{Ok::<_,()>(1)},deps![]);"));
		assert!(output.contains("use_resource_with_key(\"b\",||async{Ok::<_,()>(2)},deps![]);"));
		assert!(
			output.contains("use_resource_with_key(\"c\",||async{Ok::<_,()>(3)},deps![signal]);")
		);
	}

	#[test]
	fn explicit_dependency_matrix_converts_empty_single_and_multiple_tuples() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(a: Signal<i32>, b: Signal<i32>) {
    let _ = use_effect(|| {}, ());
    let _ = use_effect(|| {}, (a.clone(),));
    let _ = use_effect(|| {}, (a.clone(), b.clone()));
}
"#,
		));
		assert!(output.contains("use_effect(||{},deps![]);"));
		assert!(output.contains("use_effect(||{},deps![a]);"));
		assert!(output.contains("use_effect(||{},deps![a,b]);"));
	}

	#[test]
	fn block_clone_alias_maps_back_to_outer_dependency() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(original: Signal<i32>) {
    let _ = use_effect({
        let alias = original.clone();
        move || { let _ = alias.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![original]"));
		assert!(!output.contains("deps![alias]"));
	}

	#[test]
	fn unsafe_omitted_forms_keep_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect(move || { let _ = count.get(); });
    let _ = use_effect(build_callback());
    let _ = use_effect({
        let alias = count.clone();
        move || Some(move || { let _ = alias.get(); })
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 3);
		assert_eq!(output.matches("deps![count]").count(), 3);
		assert!(!output.contains("deps![alias]"));
	}

	#[test]
	fn imports_deps_in_each_inline_module_scope() {
		let output = rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn root() { let _ = use_effect(|| {}); }
mod nested {
    use reinhardt_pages::reactive::hooks::use_effect;
    fn view() { let _ = use_effect(|| {}); }
}
"#,
		);
		assert_eq!(output.matches("use reinhardt_pages::deps;").count(), 2);
	}

	#[test]
	fn qualified_hook_preserves_full_facade_path() {
		let output = compact(&rewrite(
			r#"
fn view() { let _ = reinhardt::pages::use_effect(|| {}); }
"#,
		));
		assert!(output.contains("usereinhardt::pages::deps;"));
		assert!(!output.contains("usereinhardt::deps;"));
	}

	#[test]
	fn renamed_deps_import_does_not_satisfy_bare_macro_import() {
		let output = rewrite(
			r#"
use reinhardt_pages::deps as hook_deps;
use reinhardt_pages::use_effect;
fn view() { let _ = use_effect(|| {}); }
"#,
		);
		assert!(output.contains("use reinhardt_pages::deps as hook_deps;"));
		assert!(output.contains("use reinhardt_pages::deps;"));
	}

	#[test]
	fn clone_alias_chain_resolves_to_the_outer_dependency() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let first = count.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![count]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn clone_alias_chain_accepts_a_field_place() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(state: State) {
    let _ = use_effect({
        let first = state.count.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![state.count]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn same_name_clone_shadow_resolves_through_the_previous_alias() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let first = count.clone();
        let first = first.clone();
        move || { let _ = first.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![count]"));
		assert!(!output.contains("deps![first]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn clone_aliases_snapshot_before_later_shadowing() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>, other: Signal<i32>) {
    let _ = use_effect({
        let first = count.clone();
        let second = first.clone();
        let first = other.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![count]"));
		assert!(!output.contains("},deps![other]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn clone_alias_dependents_are_invalidated_by_base_shadowing() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(state: Signal<i32>, other: Signal<i32>) {
    let _ = use_effect({
        let first = state.clone();
        let second = first.clone();
        let state = other.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains("deps![other]"));
	}

	#[test]
	fn field_alias_cycles_keep_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view() {
    let _ = use_effect({
        let first = second.field.clone();
        let second = first.clone();
        move || { let _ = first.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains("deps![second.field]"));
	}

	#[test]
	fn field_clone_aliases_snapshot_before_base_shadowing() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(state: State, other: Signal<i32>) {
    let _ = use_effect({
        let first = state.count.clone();
        let state = other.clone();
        move || { let _ = first.get(); }
    });
}
"#,
		));
		assert!(!output.contains("deps![other.count]"));
		assert_eq!(output.matches("compile_error!").count(), 1);
	}

	#[test]
	fn side_effecting_or_indexed_clone_receivers_keep_review_markers() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(signals: Vec<Signal<i32>>) {
    let _ = use_effect({
        let signal = make_signal().clone();
        move || { let _ = signal.get(); }
    });
    let _ = use_effect({
        let signal = signals[0].clone();
        move || { let _ = signal.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 2);
	}

	#[test]
	fn cyclic_clone_aliases_keep_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view() {
    let _ = use_effect({
        let first = second.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
	}

	#[test]
	fn module_alias_hook_call_restores_the_full_facade() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks as hooks;
fn view() { let _ = hooks::use_effect(|| {}); }
"#,
		));
		assert!(output.contains("usereinhardt_pages::deps;"));
		assert!(output.contains("hooks::use_effect(||{},deps![]);"));
	}

	#[test]
	fn module_name_and_self_qualified_hook_calls_restore_the_facade() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks;
fn one() { let _ = hooks::use_effect(|| {}); }
fn two() { let _ = self::hooks::use_effect(|| {}); }
"#,
		));
		assert_eq!(output.matches("usereinhardt_pages::deps;").count(), 1);
		assert_eq!(output.matches("deps![]").count(), 2);
	}

	#[test]
	fn generated_deps_import_inherits_hook_import_cfg() {
		let output = rewrite(
			r#"
#[cfg(wasm)]
use reinhardt_pages::reactive::hooks as hooks;
fn view() { let _ = hooks::use_effect(|| {}); }
"#,
		);
		assert!(output.contains("#[cfg(wasm)]\nuse reinhardt_pages::deps;"));
	}

	#[test]
	fn generated_deps_imports_deduplicate_by_cfg_and_path() {
		let output = rewrite(
			r#"
#[cfg(wasm)]
use reinhardt_pages::reactive::hooks as wasm_hooks;
#[cfg(native)]
use reinhardt_pages::reactive::hooks as native_hooks;
fn view() {
    let _ = wasm_hooks::use_effect(|| {});
    let _ = wasm_hooks::use_layout_effect(|| {});
    let _ = native_hooks::use_effect(|| {});
}
"#,
		);
		assert_eq!(
			output
				.matches("#[cfg(wasm)]\nuse reinhardt_pages::deps;")
				.count(),
			1
		);
		assert_eq!(
			output
				.matches("#[cfg(native)]\nuse reinhardt_pages::deps;")
				.count(),
			1
		);
	}

	#[test]
	fn unconditional_deps_import_supersedes_cfg_variant() {
		let output = rewrite(
			r#"
#[cfg(feature = "foo")]
use foo::reactive::hooks as wasm_hooks;
use foo::reactive::hooks as native_hooks;
fn view(signal: Signal<i32>) {
    let _ = wasm_hooks::use_effect({
        let signal = signal.clone();
        move || { let _ = signal.get(); }
    });
    let _ = native_hooks::use_effect({
        let signal = signal.clone();
        move || { let _ = signal.get(); }
    });
}
"#,
		);
		assert_eq!(output.matches("use foo::deps;").count(), 1);
		assert!(!output.contains("#[cfg(feature = \"foo\")]\nuse foo::deps;"));
	}

	#[test]
	fn same_bound_hook_imports_generate_deps_for_each_cfg_path() {
		let output = rewrite(
			r#"
#[cfg(feature = "foo")]
use foo::reactive::hooks::use_effect;
#[cfg(not(feature = "foo"))]
use bar::reactive::hooks::use_effect;
fn view(signal: Signal<i32>) {
    let _ = use_effect({
        let signal = signal.clone();
        move || { let _ = signal.get(); }
    });
}
"#,
		);
		assert!(output.contains("#[cfg(feature = \"foo\")]\nuse foo::deps;"));
		assert!(output.contains("#[cfg(not(feature = \"foo\"))]\nuse bar::deps;"));
		assert_eq!(output.matches("deps![signal]").count(), 1);
	}

	#[test]
	fn ambiguous_same_bound_hook_specs_keep_a_review_marker() {
		let output = rewrite(
			r#"
#[cfg(feature = "foo")]
use foo::reactive::hooks::use_effect as hook;
#[cfg(not(feature = "foo"))]
use bar::reactive::hooks::use_resource as hook;
fn view(signal: Signal<i32>) {
    let _ = hook({
        let signal = signal.clone();
        move || { let _ = signal.get(); }
    });
}
"#,
		);
		assert!(!output.contains("use foo::deps;"));
		assert!(!output.contains("use bar::deps;"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn shadowed_clone_alias_keeps_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let alias = count.clone();
        let alias = make_signal();
        move || { let _ = alias.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains(",deps![count]);"));
	}

	#[test]
	fn closure_local_shadow_keeps_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let alias = count.clone();
        move || {
            let alias = make_signal();
            let _ = alias.get();
        }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains(",deps![count]);"));
	}

	#[test]
	fn closure_destructure_shadow_keeps_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let alias = count.clone();
        move || {
            let (alias, _) = make_pair();
            let _ = alias.get();
        }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains(",deps![count]);"));
	}

	#[test]
	fn control_flow_pattern_shadow_keeps_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let alias = count.clone();
        move || {
            for alias in make_iter() { let _ = alias.get(); }
            if let Some(alias) = make_option() { let _ = alias.get(); }
            match make_option() {
                Some(alias) => { let _ = alias.get(); }
                None => {}
            }
        }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
		assert!(!output.contains(",deps![count]);"));
	}

	#[test]
	fn relative_reexport_does_not_generate_an_unresolved_deps_import() {
		let output = rewrite(
			r#"
mod parent {
    use reinhardt_pages::use_effect;
    mod child {
        use super::use_effect;
        fn view(signal: Signal<i32>) {
            let _ = use_effect(|| { let _ = signal.get(); });
        }
    }
}
"#,
		);
		assert!(!output.contains("use super::deps;"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn relative_reexport_tuple_is_replaced_with_a_review_marker() {
		let output = compact(&rewrite(
			r#"
mod parent {
    use reinhardt_pages::use_effect;
    mod child {
        use super::use_effect;
        fn view(signal: Signal<i32>) {
            let _ = use_effect(|| {}, (signal.clone(),));
        }
    }
}
"#,
		));
		assert!(!output.contains("use super::deps;"));
		assert!(!output.contains("(signal.clone(),)"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn unresolved_bare_hook_with_explicit_dependencies_is_not_rewritten() {
		let output = compact(&rewrite(
			r#"
fn use_effect<F, D>(callback: F, dependencies: D) {}
fn view(signal: Signal<i32>) {
    use_effect(|| {}, (signal.clone(),));
}
"#,
		));

		assert!(output.contains("use_effect(||{},(signal.clone(),));"));
		assert!(!output.contains("use reinhardt_pages::deps;"));
	}

	#[test]
	fn unresolved_bare_hook_with_omitted_dependencies_is_not_rewritten() {
		let output = compact(&rewrite(
			r#"
fn use_effect<F>(callback: F) {}
fn view() {
    use_effect(|| {});
}
"#,
		));

		assert!(output.contains("use_effect(||{});"));
		assert!(!output.contains("reinhardt_pages::deps"));
	}

	#[test]
	fn local_module_hook_does_not_generate_an_unresolved_deps_import() {
		let output = rewrite(
			r#"
mod custom {}
use custom::reactive::hooks::use_effect;
fn view(signal: Signal<i32>) {
    let _ = use_effect(|| { let _ = signal.get(); });
}
"#,
		);
		assert!(!output.contains("use custom::deps;"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn crate_local_module_hook_does_not_generate_an_unresolved_deps_import() {
		let output = rewrite(
			r#"
mod custom {}
use crate::custom::reactive::hooks::use_effect;
fn view(signal: Signal<i32>) {
    let _ = use_effect(|| { let _ = signal.get(); });
}
"#,
		);
		assert!(!output.contains("use crate::custom::deps;"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn nested_crate_local_module_hook_does_not_generate_an_unresolved_deps_import() {
		let output = rewrite(
			r#"
mod parent {
    mod custom {}
    use crate::parent::custom::reactive::hooks::use_effect;
    fn view(signal: Signal<i32>) {
        let _ = use_effect(|| { let _ = signal.get(); });
    }
}
"#,
		);
		assert!(!output.contains("use crate::parent::custom::deps;"));
		assert!(output.contains("compile_error!"));
	}

	#[test]
	fn function_local_hook_import_does_not_fall_back_to_reinhardt_pages() {
		let output = rewrite(
			r#"
mod custom {}
fn view(signal: Signal<i32>) {
    use custom::reactive::hooks::use_effect;
    let _ = use_effect({
        let signal = signal.clone();
        move || { let _ = signal.get(); }
    });
    let _ = use_effect(|| {}, (signal.clone(),));
}
"#,
		);
		assert!(!output.contains("use reinhardt_pages::deps;"));
		assert!(!output.contains("use custom::deps;"));
		assert_eq!(output.matches("compile_error!").count(), 2);
		assert!(!output.contains("(signal.clone(),)"));
	}
}
