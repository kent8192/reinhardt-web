//! Rule §6.1 (1): `tag { ident }` → `tag { {ident} }`.
//!
//! Conservative implementation — bails out on any token that could mean
//! attribute, event, nested element, component call, or method chain. The
//! resulting diff is the developer's review surface; missing transformations
//! are acceptable and caught on a second pass.

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `bare_ident` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"bare_ident"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		PageMacroBodyVisitor.visit_file_mut(&mut file);
		file
	}
}

struct PageMacroBodyVisitor;

impl VisitMut for PageMacroBodyVisitor {
	fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
		// Only target the `page!` macro (and any qualified path ending in `page`).
		if m
			.path
			.segments
			.last()
			.map(|s| s.ident == "page")
			.unwrap_or(false)
		{
			m.tokens = rewrite_page_body(m.tokens.clone());
		}
		visit_mut::visit_macro_mut(self, m);
	}
}

/// Walks the token stream of a `page!` body. Inside brace-delimited groups,
/// promotes any bare lowercase-leading identifier to a `{ident}` group.
fn rewrite_page_body(input: TokenStream) -> TokenStream {
	let mut out: Vec<TokenTree> = Vec::new();
	let mut iter = input.into_iter();
	while let Some(tt) = iter.next() {
		match tt {
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				let inner = rewrite_brace_body(g.stream());
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			}
			other => out.push(other),
		}
	}
	out.into_iter().collect()
}

/// Inside a `{ ... }` body, promote a bare ident to `{ident}` when it appears
/// in child-node position.
fn rewrite_brace_body(input: TokenStream) -> TokenStream {
	let mut out: Vec<TokenTree> = Vec::new();
	let mut iter = input.into_iter().peekable();

	while let Some(tt) = iter.next() {
		// Look for `Ident` followed by something that is NOT one of:
		//   `{` — element body
		//   `(` — component call or function call
		//   `:` — attribute syntax (`class: "x"`)
		//   `!` — macro call (`println!`)
		//   `.` — method chain or field access
		//   `,` — would already be the end of a value but still ambiguous
		//   `::` — path continuation
		// These all mean "not a bare expression in body position".
		if let TokenTree::Ident(id) = &tt
			&& starts_lowercase(&id.to_string())
		{
			let is_followed_by_continuation = match iter.peek() {
				Some(TokenTree::Group(g))
					if matches!(g.delimiter(), Delimiter::Brace | Delimiter::Parenthesis) =>
				{
					true
				}
				Some(TokenTree::Punct(p))
					if matches!(p.as_char(), ':' | '!' | '.' | ',') =>
				{
					true
				}
				_ => false,
			};

			if !is_followed_by_continuation {
				// Bare ident in body position — wrap.
				let ident = id.clone();
				let wrapped = quote! { #ident };
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, wrapped)));
				continue;
			}
		}

		// Recurse into any nested braced group (could be a child element body).
		if let TokenTree::Group(g) = &tt
			&& g.delimiter() == Delimiter::Brace
		{
			let inner = rewrite_brace_body(g.stream());
			out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			continue;
		}

		out.push(tt);
	}

	out.into_iter().collect()
}

fn starts_lowercase(s: &str) -> bool {
	s.chars()
		.next()
		.map(|c| c.is_ascii_lowercase())
		.unwrap_or(false)
}
