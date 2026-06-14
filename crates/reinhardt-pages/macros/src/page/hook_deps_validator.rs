//! Pre-codegen pass that verifies hook deps tuples cover all Signal reads
//! inside hook closures (Refs #4195 / #4721 / #4746, Manouche v2 Layer ②
//! React alignment).
//!
//! # What this checks
//!
//! For every `use_effect` / `use_layout_effect` / `use_memo` / `use_callback`
//! / `use_callback_with` call written **directly inside a `page!` body**, this
//! pass walks the hook's closure (positional arg 0) and collects the Signal
//! reads — `signal.get()`, `signal.with(...)`, `signal.into_value()`. Reads
//! through the explicit escape hatches `get_untracked` / `with_untracked` are
//! ignored. It then compares those reads against the deps tuple (positional
//! arg 1) and emits a `compile_error!` for every read whose base identifier is
//! missing from the deps tuple. This promotes the React `exhaustive-deps`
//! lint to a hard compile error (DP #4: fail early).
//!
//! # Scope and limitations
//!
//! This pass only sees hook calls that are **textually inside** a `page!`
//! invocation. Hooks called in a surrounding component `fn` body or in a
//! custom hook function — the common case in real code — are invisible to the
//! `page!` macro and therefore not checked here. The guarantee that a deps
//! tuple is present *at all* is enforced separately at the type level by the
//! `*::new_with_deps` constructor arity (a missing deps argument is `E0061`).
//!
//! The analysis is intentionally conservative: when a hook call, its closure,
//! or its deps tuple cannot be matched with confidence (for example, the deps
//! argument is a runtime expression rather than a tuple literal), the pass
//! emits nothing rather than risk a false positive. The mirror case from
//! #4721 — a dependency listed but never read — is deliberately *not* a
//! compile error: stable proc-macros have no warning channel, an unused dep is
//! at worst a redundant re-run trigger (never a correctness bug), and our read
//! detection is coarse enough that erroring would risk breaking correct code.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::Expr;
use syn::visit::{self, Visit};

use reinhardt_manouche::core::{PageElse, PageIf, PageMacro, PageNode};

use super::scope_utils::collect_pat_idents;

/// Verified hook names — kept in lockstep with the `use_*(f, deps)`
/// signatures shipped in #4195.
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
pub(crate) const ESCAPE_METHODS: &[&str] = &["get_untracked", "with_untracked"];

/// Runs the hook-deps verification pass over a parsed `PageMacro`.
///
/// Returns one `compile_error!` invocation per Signal read that is missing
/// from its enclosing hook's deps tuple, or an empty `TokenStream` when no
/// such mistake is found.
pub(crate) fn verify_hook_deps(input: &PageMacro) -> TokenStream {
	let mut diagnostics: Vec<TokenStream> = Vec::new();

	// The head expression (`page!(head = ..., || { ... })`) can also embed
	// hook calls, so scan it too.
	if let Some(head) = &input.head {
		scan_expr(head, &mut diagnostics);
	}
	for node in &input.body.nodes {
		scan_node(node, &mut diagnostics);
	}

	let mut out = TokenStream::new();
	out.extend(diagnostics);
	out
}

// --- Layer 1: walk the PageNode tree, reaching every embedded `syn::Expr` ---

fn scan_node(node: &PageNode, out: &mut Vec<TokenStream>) {
	match node {
		PageNode::Element(el) => {
			for attr in &el.attrs {
				scan_expr(&attr.value, out);
			}
			for event in &el.events {
				scan_expr(&event.handler, out);
			}
			for child in &el.children {
				scan_node(child, out);
			}
		}
		PageNode::Text(_) => {}
		PageNode::Expression(e) => scan_expr(&e.expr, out),
		PageNode::If(p) => scan_if(p, out),
		PageNode::For(p) => {
			scan_expr(&p.iter, out);
			if let Some(key) = &p.key {
				scan_expr(key, out);
			}
			for child in &p.body {
				scan_node(child, out);
			}
		}
		PageNode::Component(c) => {
			for arg in &c.args {
				scan_expr(&arg.value, out);
			}
			for event in &c.events {
				scan_expr(&event.handler, out);
			}
			if let Some(children) = &c.children {
				for child in children {
					scan_node(child, out);
				}
			}
			for slot in &c.named_slots {
				for child in &slot.children {
					scan_node(child, out);
				}
			}
		}
		PageNode::Watch(w) => scan_node(&w.expr, out),
	}
}

fn scan_if(p: &PageIf, out: &mut Vec<TokenStream>) {
	scan_expr(&p.condition, out);
	for child in &p.then_branch {
		scan_node(child, out);
	}
	if let Some(els) = &p.else_branch {
		match els {
			PageElse::Block(nodes) => {
				for child in nodes {
					scan_node(child, out);
				}
			}
			PageElse::If(inner) => scan_if(inner, out),
		}
	}
}

fn scan_expr(expr: &Expr, out: &mut Vec<TokenStream>) {
	let mut visitor = HookCallVisitor { diagnostics: out };
	visitor.visit_expr(expr);
}

// --- Layer 2: find hook calls inside an embedded expression ---

/// Walks a `syn::Expr` looking for `use_*(closure, deps)` calls.
struct HookCallVisitor<'a> {
	diagnostics: &'a mut Vec<TokenStream>,
}

impl<'ast> Visit<'ast> for HookCallVisitor<'_> {
	fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
		if is_verified_hook_call(call) && call.args.len() == 2 {
			analyze_hook(&call.args[0], &call.args[1], self.diagnostics);
		}
		// Always recurse so a hook nested inside another hook's closure is
		// discovered and validated as its own call.
		visit::visit_expr_call(self, call);
	}
}

/// Returns true when `call` is a free-function call to one of the verified
/// hooks (matched on the last path segment, so `self::use_effect(..)` counts).
fn is_verified_hook_call(call: &syn::ExprCall) -> bool {
	if let Expr::Path(p) = &*call.func
		&& let Some(seg) = p.path.segments.last()
	{
		return VERIFIED_HOOKS.contains(&seg.ident.to_string().as_str());
	}
	false
}

// --- Layer 3: compare closure reads against the deps tuple ---

/// Analyzes a single hook call: emits a `compile_error!` for each Signal read
/// in the closure whose base identifier is missing from the deps tuple.
fn analyze_hook(closure_arg: &Expr, deps_arg: &Expr, out: &mut Vec<TokenStream>) {
	// Fail open when the deps argument is not a tuple literal: we cannot prove
	// any read is missing without seeing the deps contents.
	let Some(deps) = collect_dep_bases(deps_arg) else {
		return;
	};

	let mut reads = SignalReadVisitor {
		reads: Vec::new(),
		locals_stack: Vec::new(),
	};
	reads.visit_expr(closure_arg);

	let mut reported: HashSet<String> = HashSet::new();
	for (base, span) in reads.reads {
		if deps.contains(&base) {
			continue;
		}
		// Deduplicate so multiple reads of the same signal yield one error.
		if !reported.insert(base.clone()) {
			continue;
		}
		out.push(missing_dep_error(&base, span));
	}
}

/// Collects the base identifiers of every element in a deps tuple literal.
///
/// Returns `None` (fail open) when the deps argument is not a tuple literal —
/// `()` yields an empty set (mount-only).
fn collect_dep_bases(deps: &Expr) -> Option<HashSet<String>> {
	if let Expr::Tuple(tuple) = unwrap_paren(deps) {
		let mut set = HashSet::new();
		for elem in &tuple.elems {
			if let Some(base) = base_ident_of(elem) {
				set.insert(base.to_string());
			}
		}
		Some(set)
	} else {
		None
	}
}

/// Walks a hook closure collecting Signal reads, tracking lexical scopes so
/// that locally bound identifiers do not count as reads.
struct SignalReadVisitor {
	reads: Vec<(String, Span)>,
	locals_stack: Vec<HashSet<String>>,
}

impl SignalReadVisitor {
	fn is_shadowed(&self, name: &str) -> bool {
		self.locals_stack.iter().any(|s| s.contains(name))
	}
}

impl<'ast> Visit<'ast> for SignalReadVisitor {
	fn visit_expr_method_call(&mut self, mc: &'ast syn::ExprMethodCall) {
		let method = mc.method.to_string();
		let is_read = if ESCAPE_METHODS.contains(&method.as_str()) {
			// Explicit opt-out: read without subscribing.
			false
		} else if method == "get" || method == "into_value" {
			// Zero-argument accessors. `map.get(&key)` (one arg) is not a read.
			mc.args.is_empty()
		} else {
			method == "with"
		};

		if is_read && let Some(ident) = base_ident_of(&mc.receiver) {
			let name = ident.to_string();
			// `self.signal.get()` cannot be named in a deps tuple; skip.
			if name != "self" && !self.is_shadowed(&name) {
				self.reads.push((name, ident.span()));
			}
		}

		visit::visit_expr_method_call(self, mc);
	}

	fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
		// A nested hook call is validated independently; do not attribute its
		// closure's reads to the enclosing hook.
		if is_verified_hook_call(call) {
			return;
		}
		visit::visit_expr_call(self, call);
	}

	fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
		let mut locals = HashSet::new();
		for input in &c.inputs {
			collect_pat_idents(input, &mut locals);
		}
		self.locals_stack.push(locals);
		visit::visit_expr_closure(self, c);
		self.locals_stack.pop();
	}

	fn visit_expr_let(&mut self, l: &'ast syn::ExprLet) {
		// `if let pat = expr` / `while let pat = expr` in expression position.
		let mut locals = HashSet::new();
		collect_pat_idents(&l.pat, &mut locals);
		self.locals_stack.push(locals);
		visit::visit_expr_let(self, l);
		self.locals_stack.pop();
	}

	fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
		// `if let pat = expr { body }`: pattern bindings are in scope for the
		// then-branch only, NOT for the matched expression or the else branch.
		if let syn::Expr::Let(let_expr) = &*i.cond {
			let mut locals = HashSet::new();
			collect_pat_idents(&let_expr.pat, &mut locals);
			self.visit_expr(&let_expr.expr);
			self.locals_stack.push(locals);
			self.visit_block(&i.then_branch);
			self.locals_stack.pop();
			if let Some((_, else_branch)) = &i.else_branch {
				self.visit_expr(else_branch);
			}
		} else {
			visit::visit_expr_if(self, i);
		}
	}

	fn visit_arm(&mut self, a: &'ast syn::Arm) {
		// `match expr { pat => body }`: pat bindings are in scope for the arm.
		let mut locals = HashSet::new();
		collect_pat_idents(&a.pat, &mut locals);
		self.locals_stack.push(locals);
		if let Some((_, guard)) = &a.guard {
			self.visit_expr(guard);
		}
		self.visit_expr(&a.body);
		self.locals_stack.pop();
	}

	fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
		// `for pat in iter { body }`: pat bindings are in scope for the body.
		self.visit_expr(&f.expr);
		let mut locals = HashSet::new();
		collect_pat_idents(&f.pat, &mut locals);
		self.locals_stack.push(locals);
		self.visit_block(&f.body);
		self.locals_stack.pop();
	}

	fn visit_block(&mut self, b: &'ast syn::Block) {
		// Walk statements in order so each `let pat = ...;` extends scope from
		// the next statement onward.
		let mut pushed = 0_usize;
		for stmt in &b.stmts {
			match stmt {
				syn::Stmt::Local(local) => {
					// Visit the initializer first (new bindings not yet in scope).
					if let Some(init) = &local.init {
						self.visit_expr(&init.expr);
						if let Some((_, diverge)) = &init.diverge {
							self.visit_expr(diverge);
						}
						// The idiomatic clone prelude `let count = count.clone();`
						// rebinds the SAME signal to the same name. Treat such a
						// self-rebind as an alias (do not shadow) so reads of the
						// rebound name are still checked against the deps tuple.
						// Any other initializer is a genuine shadow.
						if is_self_rebind(&local.pat, &init.expr) {
							continue;
						}
					}
					let mut locals = HashSet::new();
					collect_pat_idents(&local.pat, &mut locals);
					self.locals_stack.push(locals);
					pushed += 1;
				}
				syn::Stmt::Item(_) => {}
				syn::Stmt::Expr(e, _) => self.visit_expr(e),
				syn::Stmt::Macro(m) => visit::visit_stmt_macro(self, m),
			}
		}
		for _ in 0..pushed {
			self.locals_stack.pop();
		}
	}
}

/// Returns true when `pat` is a single identifier `x` and `init` is an
/// expression whose base identifier is also `x` (e.g. `let count = count.clone()`).
fn is_self_rebind(pat: &syn::Pat, init: &Expr) -> bool {
	if let syn::Pat::Ident(pi) = pat
		&& let Some(base) = base_ident_of(init)
	{
		return *base == pi.ident;
	}
	false
}

/// Resolves the left-most single path identifier of a receiver chain, e.g.
/// `count.get()` → `count`, `count.signal.get()` → `count`. Returns `None`
/// when the head is not a bare identifier (multi-segment path, index, call
/// result), so such reads are simply left unchecked (fail open).
fn base_ident_of(e: &Expr) -> Option<&syn::Ident> {
	match e {
		Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
			Some(&p.path.segments[0].ident)
		}
		Expr::MethodCall(mc) => base_ident_of(&mc.receiver),
		Expr::Field(f) => base_ident_of(&f.base),
		Expr::Paren(p) => base_ident_of(&p.expr),
		Expr::Reference(r) => base_ident_of(&r.expr),
		Expr::Try(t) => base_ident_of(&t.expr),
		_ => None,
	}
}

/// Unwraps nested parentheses around an expression.
fn unwrap_paren(e: &Expr) -> &Expr {
	match e {
		Expr::Paren(p) => unwrap_paren(&p.expr),
		_ => e,
	}
}

/// Builds the missing-dep diagnostic, spanned at the read site.
fn missing_dep_error(base: &str, span: Span) -> TokenStream {
	let msg = format!(
		"`{base}` is read inside this hook closure but is not listed in its deps tuple.\n\n\
		 help: add `{base}.clone()` to the deps tuple, e.g. `({base}.clone(),)`\n\
		 note: if this read is intentional and should NOT trigger re-runs, use \
		 `{base}.get_untracked()` (or `with_untracked`) to opt out of dependency tracking."
	);
	quote_spanned! {span=>
		::core::compile_error!(#msg);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use quote::quote;
	use rstest::rstest;

	/// Parses a `page!` body and returns the verification output as a string.
	fn diagnostics(input: TokenStream) -> String {
		let ast: PageMacro = syn::parse2(input).expect("page! body should parse");
		verify_hook_deps(&ast).to_string()
	}

	#[rstest]
	fn missing_dep_inside_page_emits_error() {
		// Arrange
		let input = quote! {
			|count: Signal<i32>| {
				p { {
					use_effect(
						{
							let count = count.clone();
							move || { let _ = count.get(); None::<fn()> }
						},
						(),
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert!(
			out.contains("compile_error"),
			"a missing dep must emit compile_error, got: {out}"
		);
		assert!(
			out.contains("count"),
			"diagnostic should name the missing dep"
		);
	}

	#[rstest]
	fn covered_dep_is_silent() {
		// Arrange
		let input = quote! {
			|count: Signal<i32>| {
				p { {
					use_effect(
						{
							let count = count.clone();
							move || { let _ = count.get(); None::<fn()> }
						},
						(count.clone(),),
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert_eq!(out, "", "a covered dep must produce no diagnostics");
	}

	#[rstest]
	fn get_untracked_is_silent() {
		// Arrange
		let input = quote! {
			|count: Signal<i32>| {
				p { {
					use_effect(
						{
							let count = count.clone();
							move || { let _ = count.get_untracked(); None::<fn()> }
						},
						(),
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert_eq!(out, "", "an untracked read must produce no diagnostics");
	}

	#[rstest]
	fn non_tuple_deps_fails_open() {
		// Arrange — deps is a runtime expression, not a tuple literal.
		let input = quote! {
			|count: Signal<i32>, my_deps: Deps| {
				p { {
					use_effect(
						{
							let count = count.clone();
							move || { let _ = count.get(); None::<fn()> }
						},
						my_deps,
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert_eq!(out, "", "non-tuple deps must fail open (no diagnostics)");
	}

	#[rstest]
	fn extra_dep_is_not_flagged() {
		// Arrange — `count` is listed but never read; this must NOT error.
		let input = quote! {
			|count: Signal<i32>| {
				p { {
					use_effect(move || { None::<fn()> }, (count.clone(),));
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert_eq!(out, "", "an unread dep must not be a compile error");
	}

	#[rstest]
	fn shadowed_local_is_not_a_read() {
		// Arrange — `value` is a fresh local, not a signal dependency.
		let input = quote! {
			|count: Signal<i32>| {
				p { {
					use_effect(
						move || {
							let value = make();
							let _ = value.get();
							None::<fn()>
						},
						(),
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert
		assert_eq!(out, "", "a read of a shadowing local must not be flagged");
	}

	#[rstest]
	fn nested_hook_is_validated_independently() {
		// Arrange — outer effect covers `count`; inner memo is missing `other`.
		let input = quote! {
			|count: Signal<i32>, other: Signal<i32>| {
				p { {
					use_effect(
						{
							let count = count.clone();
							let other = other.clone();
							move || {
								let _ = use_memo(
									{
										let other = other.clone();
										move || other.get()
									},
									(),
								);
								let _ = count.get();
								None::<fn()>
							}
						},
						(count.clone(),),
					);
					"x"
				} }
			}
		};

		// Act
		let out = diagnostics(input);

		// Assert — the inner memo's missing `other` is flagged; the outer
		// effect's covered `count` is not.
		assert!(
			out.contains("other"),
			"inner memo's missing dep must be flagged"
		);
		assert!(
			out.matches("compile_error").count() == 1,
			"exactly one diagnostic expected, got: {out}"
		);
	}
}
