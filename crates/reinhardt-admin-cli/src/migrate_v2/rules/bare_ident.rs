//! Rule §6.1 (1): `tag { ident }` → `tag { {ident} }`.
//!
//! Conservative implementation — bails out on any token that could mean
//! attribute, event, nested element, component call, or method chain. The
//! resulting diff is the developer's review surface; missing transformations
//! are acceptable and caught on a second pass.

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use std::iter::Peekable;
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
		if m.path
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
	for tt in input {
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
	let mut previous_token_can_own_brace_body = false;

	while let Some(tt) = iter.next() {
		if let TokenTree::Ident(id) = &tt {
			match id.to_string().as_str() {
				"if" | "for" | "while" => {
					out.push(tt);
					push_control_prefix_and_rewrite_body(&mut iter, &mut out);
					previous_token_can_own_brace_body = false;
					continue;
				}
				"match" => {
					out.push(tt);
					push_match_prefix_and_rewrite_body(&mut iter, &mut out);
					previous_token_can_own_brace_body = false;
					continue;
				}
				"let" => {
					out.push(tt);
					push_statement_until_semicolon(&mut iter, &mut out);
					previous_token_can_own_brace_body = false;
					continue;
				}
				"else" => {
					out.push(tt);
					if let Some(TokenTree::Ident(next)) = iter.peek()
						&& next == "if"
					{
						let if_token = iter.next().expect("peeked token disappeared");
						out.push(if_token);
						push_control_prefix_and_rewrite_body(&mut iter, &mut out);
					} else {
						push_immediate_body_if_present(&mut iter, &mut out);
					}
					previous_token_can_own_brace_body = false;
					continue;
				}
				"loop" | "unsafe" | "async" => {
					out.push(tt);
					push_immediate_body_if_present(&mut iter, &mut out);
					previous_token_can_own_brace_body = false;
					continue;
				}
				_ => {}
			}
		}

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
			&& !is_reserved_keyword(&id.to_string())
		{
			let is_followed_by_continuation = match iter.peek() {
				Some(TokenTree::Group(g))
					if matches!(
						g.delimiter(),
						Delimiter::Brace | Delimiter::Parenthesis | Delimiter::Bracket
					) =>
				{
					true
				}
				Some(TokenTree::Punct(p)) if matches!(p.as_char(), ':' | '!' | '.' | ',') => true,
				_ => false,
			};

			if !is_followed_by_continuation {
				// Bare ident in body position — wrap.
				let ident = id.clone();
				let wrapped = quote! { #ident };
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, wrapped)));
				previous_token_can_own_brace_body = false;
				continue;
			}
		}

		// Recurse into any nested braced group (could be a child element body) —
		// UNLESS it is a standalone expression slot created by a prior run.
		// Element/control-flow bodies are owned by the previous token, while
		// expression slots appear as standalone brace groups in child position.
		if let TokenTree::Group(g) = &tt
			&& g.delimiter() == Delimiter::Brace
		{
			if previous_token_can_own_brace_body {
				let inner = rewrite_brace_body(g.stream());
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			} else {
				out.push(tt);
			}
			previous_token_can_own_brace_body = false;
			continue;
		}

		previous_token_can_own_brace_body = match &tt {
			TokenTree::Ident(id) => !is_reserved_keyword(&id.to_string()),
			TokenTree::Punct(p) => matches!(p.as_char(), ')' | ']'),
			_ => false,
		};
		out.push(tt);
	}

	out.into_iter().collect()
}

fn push_control_prefix_and_rewrite_body(
	iter: &mut Peekable<proc_macro2::token_stream::IntoIter>,
	out: &mut Vec<TokenTree>,
) {
	for next in iter.by_ref() {
		if let TokenTree::Group(g) = &next
			&& g.delimiter() == Delimiter::Brace
		{
			let inner = rewrite_brace_body(g.stream());
			out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			return;
		}
		out.push(next);
	}
}

fn push_match_prefix_and_rewrite_body(
	iter: &mut Peekable<proc_macro2::token_stream::IntoIter>,
	out: &mut Vec<TokenTree>,
) {
	for next in iter.by_ref() {
		if let TokenTree::Group(g) = &next
			&& g.delimiter() == Delimiter::Brace
		{
			let inner = rewrite_match_body(g.stream());
			out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
			return;
		}
		out.push(next);
	}
}

fn rewrite_match_body(input: TokenStream) -> TokenStream {
	let mut out: Vec<TokenTree> = Vec::new();
	let mut arm_value: Vec<TokenTree> = Vec::new();
	let mut iter = input.into_iter().peekable();
	let mut in_arm_value = false;

	while let Some(tt) = iter.next() {
		if !in_arm_value {
			if is_fat_arrow_start(&tt, iter.peek()) {
				out.push(tt);
				if let Some(next) = iter.next() {
					out.push(next);
				}
				in_arm_value = true;
			} else {
				out.push(tt);
			}
			continue;
		}

		if is_top_level_comma(&tt) {
			out.extend(rewrite_brace_body(arm_value.into_iter().collect()));
			arm_value = Vec::new();
			out.push(tt);
			in_arm_value = false;
		} else {
			arm_value.push(tt);
		}
	}

	if in_arm_value {
		out.extend(rewrite_brace_body(arm_value.into_iter().collect()));
	}

	out.into_iter().collect()
}

fn is_fat_arrow_start(tt: &TokenTree, next: Option<&TokenTree>) -> bool {
	matches!(tt, TokenTree::Punct(p) if p.as_char() == '=')
		&& matches!(next, Some(TokenTree::Punct(p)) if p.as_char() == '>')
}

fn is_top_level_comma(tt: &TokenTree) -> bool {
	matches!(tt, TokenTree::Punct(p) if p.as_char() == ',')
}

fn push_statement_until_semicolon(
	iter: &mut Peekable<proc_macro2::token_stream::IntoIter>,
	out: &mut Vec<TokenTree>,
) {
	let mut initializer: Vec<TokenTree> = Vec::new();
	let mut in_initializer = false;

	for next in iter.by_ref() {
		let is_semicolon = matches!(&next, TokenTree::Punct(p) if p.as_char() == ';');
		let is_assignment = matches!(&next, TokenTree::Punct(p) if p.as_char() == '=');

		if in_initializer {
			if is_semicolon {
				out.extend(rewrite_let_initializer(initializer.into_iter().collect()));
				out.push(next);
				return;
			}
			initializer.push(next);
			continue;
		}

		out.push(next);
		if is_assignment {
			in_initializer = true;
		}
		if is_semicolon {
			return;
		}
	}

	if in_initializer {
		out.extend(rewrite_let_initializer(initializer.into_iter().collect()));
	}
}

fn rewrite_let_initializer(input: TokenStream) -> TokenStream {
	let mut out: Vec<TokenTree> = Vec::new();
	let mut previous_token_can_own_brace_body = false;

	for tt in input {
		match tt {
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				let inner = if previous_token_can_own_brace_body {
					rewrite_brace_body(g.stream())
				} else {
					rewrite_let_initializer(g.stream())
				};
				out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
				previous_token_can_own_brace_body = false;
			}
			TokenTree::Group(g) => {
				let delimiter = g.delimiter();
				let inner = rewrite_let_initializer(g.stream());
				out.push(TokenTree::Group(Group::new(delimiter, inner)));
				previous_token_can_own_brace_body = false;
			}
			other => {
				previous_token_can_own_brace_body = token_can_own_let_initializer_body(&other);
				out.push(other);
			}
		}
	}

	out.into_iter().collect()
}

fn token_can_own_let_initializer_body(tt: &TokenTree) -> bool {
	matches!(
		tt,
		TokenTree::Ident(id)
			if starts_lowercase(&id.to_string()) && !is_reserved_keyword(&id.to_string())
	)
}

fn push_immediate_body_if_present(
	iter: &mut Peekable<proc_macro2::token_stream::IntoIter>,
	out: &mut Vec<TokenTree>,
) {
	if let Some(TokenTree::Group(g)) = iter.peek()
		&& g.delimiter() == Delimiter::Brace
	{
		let body = match iter.next() {
			Some(TokenTree::Group(g)) => g,
			_ => unreachable!("peek matched but next did not"),
		};
		let inner = rewrite_brace_body(body.stream());
		out.push(TokenTree::Group(Group::new(Delimiter::Brace, inner)));
	}
}

fn starts_lowercase(s: &str) -> bool {
	s.chars()
		.next()
		.map(|c| c.is_ascii_lowercase())
		.unwrap_or(false)
}

/// Rust reserved keywords that can legally appear at the head of an
/// expression / control-flow construct inside a `page!` body. We must
/// never wrap these in braces — `{ if } cond { ... }` is a parse error.
fn is_reserved_keyword(s: &str) -> bool {
	matches!(
		s,
		"if" | "else"
			| "match" | "for"
			| "while" | "loop"
			| "let" | "return"
			| "break" | "continue"
			| "move" | "ref"
			| "mut" | "async"
			| "await" | "yield"
			| "do" | "in"
			| "as" | "where"
			| "use" | "fn"
			| "true" | "false"
			| "self" | "Self"
			| "super" | "crate"
			| "impl" | "trait"
			| "struct"
			| "enum" | "type"
			| "const" | "static"
			| "pub" | "mod"
			| "unsafe"
			| "extern"
	)
}
