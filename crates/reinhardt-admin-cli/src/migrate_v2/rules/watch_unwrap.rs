//! Rule §6.1 (2)+(3): `watch { body }` and `#reactive { body }` →
//! splice `body` in place, dropping the wrapper.
//!
//! `#reactive` was an early-design wrapper that never reached `main`, but
//! the rule keeps a defensive arm so any in-flight branch picking up the
//! codemod still gets a clean result.

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `watch_unwrap` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"watch_unwrap"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		PageMacroBodyVisitor.visit_file_mut(&mut file);
		file
	}
}

struct PageMacroBodyVisitor;

impl VisitMut for PageMacroBodyVisitor {
	fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
		if m.path
			.segments
			.last()
			.map(|s| s.ident == "page")
			.unwrap_or(false)
		{
			m.tokens = unwrap_watch(m.tokens.clone());
		}
		visit_mut::visit_macro_mut(self, m);
	}
}

fn unwrap_watch(input: TokenStream) -> TokenStream {
	let mut out: Vec<TokenTree> = Vec::new();
	let mut iter = input.into_iter().peekable();

	while let Some(tt) = iter.next() {
		match &tt {
			// `watch` followed by a brace group → splice body.
			TokenTree::Ident(id) if id == "watch" => {
				if let Some(TokenTree::Group(g)) = iter.peek()
					&& g.delimiter() == Delimiter::Brace
				{
					let body = match iter.next() {
						Some(TokenTree::Group(g)) => g.stream(),
						_ => unreachable!("peek matched but next did not"),
					};
					out.extend(unwrap_watch(body));
					continue;
				}
				out.push(tt);
			}
			// `#reactive` (Punct '#' then Ident "reactive") followed by a brace.
			TokenTree::Punct(p) if p.as_char() == '#' => {
				let next = iter.peek().cloned();
				if let Some(TokenTree::Ident(id2)) = next
					&& id2 == "reactive"
				{
					let _ = iter.next(); // consume "reactive"
					if let Some(TokenTree::Group(g)) = iter.peek()
						&& g.delimiter() == Delimiter::Brace
					{
						let body = match iter.next() {
							Some(TokenTree::Group(g)) => g.stream(),
							_ => unreachable!("peek matched but next did not"),
						};
						out.extend(unwrap_watch(body));
						continue;
					}
				}
				out.push(tt);
			}
			// Recurse into any other brace group.
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				let inner = unwrap_watch(g.stream());
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			}
			_ => out.push(tt),
		}
	}

	out.into_iter().collect()
}
