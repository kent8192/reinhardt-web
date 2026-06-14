//! Shared scope-tracking helpers for the `page!` macro validation passes.
//!
//! Both the capture-discipline validator and the hook-deps validator need to
//! enumerate the identifiers a pattern binds (closure params, `let` bindings,
//! `for` loop variables, match-arm patterns) so they can model lexical scopes
//! and decide whether a referenced identifier is a local binding.

use std::collections::HashSet;

/// Recursively collects identifier names bound by a pattern.
///
/// Handles the pattern shapes that can appear in closure params, `let`
/// statements, `for` loops, and match arms inside a `page!` body. Unsupported
/// shapes contribute no bindings (a conservative choice: an unrecognized
/// pattern simply does not shadow anything).
pub(crate) fn collect_pat_idents(p: &syn::Pat, out: &mut HashSet<String>) {
	match p {
		syn::Pat::Ident(pi) => {
			out.insert(pi.ident.to_string());
		}
		syn::Pat::Tuple(t) => {
			for el in &t.elems {
				collect_pat_idents(el, out);
			}
		}
		syn::Pat::TupleStruct(ts) => {
			for el in &ts.elems {
				collect_pat_idents(el, out);
			}
		}
		syn::Pat::Struct(ps) => {
			for f in &ps.fields {
				collect_pat_idents(&f.pat, out);
			}
		}
		syn::Pat::Reference(r) => collect_pat_idents(&r.pat, out),
		syn::Pat::Type(t) => collect_pat_idents(&t.pat, out),
		syn::Pat::Or(o) => {
			for case in &o.cases {
				collect_pat_idents(case, out);
			}
		}
		syn::Pat::Slice(s) => {
			for el in &s.elems {
				collect_pat_idents(el, out);
			}
		}
		syn::Pat::Paren(p) => collect_pat_idents(&p.pat, out),
		_ => {}
	}
}
