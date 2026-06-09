//! Compile-free static `page!` hot patch support for Pages development.

use std::path::{Path, PathBuf};

/// Render a static `page!(|| { ... })` edit into an HMR HTML replacement.
///
/// This deliberately accepts only a narrow static subset: string attributes,
/// string text nodes, and nested lowercase HTML elements. Dynamic Rust
/// expressions, event handlers, control flow, and components return `None` so
/// the normal WASM rebuild path remains authoritative.
pub fn render_static_page_patch(paths: &[PathBuf]) -> Option<String> {
	let path = only_rust_path(paths)?;
	let source = std::fs::read_to_string(path).ok()?;
	render_static_page_source(&source)
}

fn only_rust_path(paths: &[PathBuf]) -> Option<&Path> {
	if paths.len() != 1 {
		return None;
	}
	let path = paths[0].as_path();
	if path.extension()? != "rs" {
		return None;
	}
	Some(path)
}

fn render_static_page_source(source: &str) -> Option<String> {
	let body = find_static_page_body(source)?;
	let mut parser = Parser::new(body);
	let html = parser.parse_nodes_until_end().ok()?;
	if html.trim().is_empty() {
		return None;
	}
	Some(html)
}

fn find_static_page_body(source: &str) -> Option<&str> {
	let page_start = source.find("page!")?;
	let after_macro = source[page_start + "page!".len()..].trim_start();
	let inner = after_macro.strip_prefix('(')?;
	let closure_start = inner.find("||")?;
	let after_closure = inner[closure_start + 2..].trim_start();
	let body_start = after_closure.find('{')?;
	let body = &after_closure[body_start + 1..];
	let body_end = matching_brace_offset(body)?;
	Some(&body[..body_end])
}

fn matching_brace_offset(input: &str) -> Option<usize> {
	let mut depth = 1usize;
	let mut in_string = false;
	let mut escaped = false;

	for (offset, ch) in input.char_indices() {
		if in_string {
			if escaped {
				escaped = false;
			} else if ch == '\\' {
				escaped = true;
			} else if ch == '"' {
				in_string = false;
			}
			continue;
		}

		match ch {
			'"' => in_string = true,
			'{' => depth += 1,
			'}' => {
				depth = depth.checked_sub(1)?;
				if depth == 0 {
					return Some(offset);
				}
			}
			_ => {}
		}
	}

	None
}

struct Parser<'a> {
	input: &'a str,
	pos: usize,
}

impl<'a> Parser<'a> {
	fn new(input: &'a str) -> Self {
		Self { input, pos: 0 }
	}

	fn parse_nodes_until_end(&mut self) -> Result<String, ()> {
		let mut html = String::new();
		loop {
			self.skip_ws_and_commas();
			if self.is_eof() {
				return Ok(html);
			}
			if self.peek_char() == Some('}') {
				return Err(());
			}
			html.push_str(&self.parse_node()?);
		}
	}

	fn parse_node(&mut self) -> Result<String, ()> {
		self.skip_ws_and_commas();
		match self.peek_char() {
			Some('"') => Ok(escape_text(&self.parse_string_literal()?)),
			Some('@') | Some('{') => Err(()),
			Some(ch) if is_ident_start(ch) => self.parse_element(),
			_ => Err(()),
		}
	}

	fn parse_element(&mut self) -> Result<String, ()> {
		let tag = self.parse_ident()?;
		if matches!(tag.as_str(), "if" | "for" | "watch") || !is_html_tag_name(&tag) {
			return Err(());
		}
		self.skip_ws_and_commas();
		if self.bump_char() != Some('{') {
			return Err(());
		}

		let mut attrs = Vec::new();
		let mut children = String::new();
		loop {
			self.skip_ws_and_commas();
			if self.is_eof() {
				return Err(());
			}
			if self.peek_char() == Some('}') {
				self.bump_char();
				break;
			}
			if self.peek_char() == Some('"') {
				children.push_str(&escape_text(&self.parse_string_literal()?));
				continue;
			}
			if matches!(self.peek_char(), Some('@') | Some('{')) {
				return Err(());
			}

			let checkpoint = self.pos;
			let ident = self.parse_ident()?;
			self.skip_ws();
			match self.peek_char() {
				Some(':') => {
					self.bump_char();
					self.skip_ws();
					let value = self.parse_string_literal()?;
					attrs.push((ident.replace('_', "-"), value));
					self.skip_ws_and_commas();
				}
				Some('{') => {
					self.pos = checkpoint;
					children.push_str(&self.parse_element()?);
				}
				_ => return Err(()),
			}
		}

		let mut html = String::new();
		html.push('<');
		html.push_str(&tag);
		for (name, value) in attrs {
			html.push(' ');
			html.push_str(&name);
			html.push_str("=\"");
			html.push_str(&escape_attr(&value));
			html.push('"');
		}
		html.push('>');
		html.push_str(&children);
		html.push_str("</");
		html.push_str(&tag);
		html.push('>');
		Ok(html)
	}

	fn parse_ident(&mut self) -> Result<String, ()> {
		let start = self.pos;
		let Some(ch) = self.peek_char() else {
			return Err(());
		};
		if !is_ident_start(ch) {
			return Err(());
		}
		self.bump_char();
		while matches!(self.peek_char(), Some(ch) if is_ident_continue(ch)) {
			self.bump_char();
		}
		Ok(self.input[start..self.pos].to_string())
	}

	fn parse_string_literal(&mut self) -> Result<String, ()> {
		let start = self.pos;
		if self.bump_char() != Some('"') {
			return Err(());
		}
		let mut escaped = false;
		while let Some(ch) = self.bump_char() {
			if escaped {
				escaped = false;
				continue;
			}
			if ch == '\\' {
				escaped = true;
				continue;
			}
			if ch == '"' {
				let literal = &self.input[start..self.pos];
				return syn::parse_str::<syn::LitStr>(literal)
					.map(|lit| lit.value())
					.map_err(|_| ());
			}
		}
		Err(())
	}

	fn skip_ws_and_commas(&mut self) {
		while matches!(self.peek_char(), Some(ch) if ch.is_whitespace() || ch == ',') {
			self.bump_char();
		}
	}

	fn skip_ws(&mut self) {
		while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
			self.bump_char();
		}
	}

	fn is_eof(&self) -> bool {
		self.pos >= self.input.len()
	}

	fn peek_char(&self) -> Option<char> {
		self.input[self.pos..].chars().next()
	}

	fn bump_char(&mut self) -> Option<char> {
		let ch = self.peek_char()?;
		self.pos += ch.len_utf8();
		Some(ch)
	}
}

fn is_ident_start(ch: char) -> bool {
	ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
	ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_html_tag_name(name: &str) -> bool {
	name.chars()
		.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

fn escape_text(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
	escape_text(value).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn renders_static_page_macro() {
		let source = r#"
			fn home_page() -> Page {
				page!(|| {
					div {
						id: "route-home",
						a {
							href: "/login",
							id: "go-to-login",
							"Go to login"
						}
					}
				})()
			}
		"#;

		let html = render_static_page_source(source).expect("static page should render");

		assert_eq!(
			html,
			r#"<div id="route-home"><a href="/login" id="go-to-login">Go to login</a></div>"#
		);
	}

	#[rstest]
	fn rejects_dynamic_expression() {
		let source = r#"
			fn home_page(name: String) -> Page {
				page!(|| {
					div { { name } }
				})()
			}
		"#;

		assert!(render_static_page_source(source).is_none());
	}

	#[rstest]
	fn rejects_event_handler() {
		let source = r#"
			fn home_page() -> Page {
				page!(|| {
					button {
						@click: |_| {},
						"Save"
					}
				})()
			}
		"#;

		assert!(render_static_page_source(source).is_none());
	}

	#[rstest]
	fn escapes_text_and_attrs() {
		let source = r#"
			fn home_page() -> Page {
				page!(|| {
					div {
						title: "A \"quote\" & more",
						"<unsafe>"
					}
				})()
			}
		"#;

		let html = render_static_page_source(source).expect("static page should render");

		assert_eq!(
			html,
			r#"<div title="A &quot;quote&quot; &amp; more">&lt;unsafe&gt;</div>"#
		);
	}
}
