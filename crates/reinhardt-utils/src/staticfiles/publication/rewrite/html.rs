//! Token-based HTML asset attributes and inline CSS/module references.

use super::{Site, apply_edits, css, javascript, reference, replacement};
use crate::staticfiles::publication::{AssetBuildError, AssetReference};
use html5ever::buffer_queue::BufferQueue;
use html5ever::tokenizer::states::RawKind;
use html5ever::tokenizer::{TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct Sink(RefCell<Vec<Token>>);

impl TokenSink for Sink {
	type Handle = ();
	fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
		let result = match &token {
			Token::TagToken(tag) if tag.kind == TagKind::StartTag => match tag.name.as_ref() {
				"script" => TokenSinkResult::RawData(RawKind::ScriptData),
				"style" | "xmp" | "iframe" | "noembed" | "noframes" => {
					TokenSinkResult::RawData(RawKind::Rawtext)
				}
				"title" | "textarea" => TokenSinkResult::RawData(RawKind::Rcdata),
				_ => TokenSinkResult::Continue,
			},
			_ => TokenSinkResult::Continue,
		};
		let mut tokens = self.0.borrow_mut();
		if let Token::CharacterTokens(text) = &token
			&& let Some(Token::CharacterTokens(previous)) = tokens.last_mut()
		{
			previous.push_tendril(text);
			return result;
		}
		tokens.push(token);
		result
	}
}

fn tokenize(source: &str) -> Vec<Token> {
	let tokenizer = Tokenizer::new(Sink::default(), TokenizerOpts::default());
	let input = BufferQueue::default();
	input.push_back(source.into());
	let _ = tokenizer.feed(&input);
	tokenizer.end();
	tokenizer.sink.0.into_inner()
}

pub(super) fn analyze(logical: &str, source: &str) -> Result<Vec<AssetReference>, AssetBuildError> {
	let tokens = tokenize(source);
	let mut references = Vec::new();
	let mut inline = None;
	for (index, token) in tokens.iter().enumerate() {
		match token {
			Token::TagToken(tag) if tag.kind == TagKind::StartTag => {
				let name = tag.name.as_ref();
				let attribute = |name: &str| {
					tag.attrs
						.iter()
						.find(|a| a.name.local.as_ref() == name)
						.map(|a| a.value.as_ref())
				};
				if name == "base" && attribute("href").is_some() {
					return Err(AssetBuildError::input(
						logical,
						"HTML base href changes asset resolution; remove it or use a custom processor",
					));
				}
				if name == "script"
					&& attribute("type").is_some_and(|v| v.eq_ignore_ascii_case("importmap"))
				{
					return Err(AssetBuildError::input(
						logical,
						"HTML import maps require bundling or a custom processor",
					));
				}
				inline = if name == "style" {
					Some("css")
				} else if name == "script"
					&& attribute("type").is_some_and(|v| v.eq_ignore_ascii_case("module"))
					&& attribute("src").is_none()
				{
					Some("js")
				} else {
					None
				};
				for (attr_index, attr) in tag.attrs.iter().enumerate() {
					let attr_name = attr.name.local.as_ref();
					if attr_name == "style" {
						add_inline(
							&mut references,
							css::analyze(logical, &attr.value)?,
							index,
							Some(attr_index),
						)?;
						continue;
					}
					if attr_name == "srcset" && matches!(name, "img" | "source") {
						for (candidate, (start, end)) in
							srcset_urls(&attr.value).into_iter().enumerate()
						{
							if let Some(r) = html_reference(
								logical,
								&attr.value[start..end],
								Site::HtmlAttribute {
									token: index,
									attribute: attr_index,
									candidate: Some(candidate),
								},
							)? {
								references.push(r);
							}
						}
						continue;
					}
					let asset = match attr_name {
						"src" => matches!(
							name,
							"script"
								| "img" | "audio" | "video"
								| "source" | "track" | "embed"
								| "input"
						),
						"href" | "xlink:href" => {
							matches!(name, "use" | "image")
								|| name == "link"
									&& attribute("rel").is_some_and(|value| {
										value.split_ascii_whitespace().any(|rel| {
											[
												"stylesheet",
												"icon",
												"preload",
												"modulepreload",
												"apple-touch-icon",
												"manifest",
											]
											.iter()
											.any(|v| rel.eq_ignore_ascii_case(v))
										})
									})
						}
						"poster" => name == "video",
						"data" => name == "object",
						_ => false,
					};
					if asset
						&& let Some(r) = html_reference(
							logical,
							&attr.value,
							Site::HtmlAttribute {
								token: index,
								attribute: attr_index,
								candidate: None,
							},
						)? {
						references.push(r);
					}
				}
			}
			Token::TagToken(tag) if tag.kind == TagKind::EndTag => inline = None,
			Token::CharacterTokens(text) => match inline {
				Some("css") => {
					add_inline(&mut references, css::analyze(logical, text)?, index, None)?
				}
				Some("js") => add_inline(
					&mut references,
					javascript::analyze(logical, text)?,
					index,
					None,
				)?,
				_ => {}
			},
			_ => {}
		}
	}
	Ok(references)
}

fn html_reference(
	logical: &str,
	value: &str,
	site: Site,
) -> Result<Option<AssetReference>, AssetBuildError> {
	let trimmed = value.trim();
	if let Some(expression) = trimmed
		.strip_prefix("{{")
		.and_then(|v| v.strip_suffix("}}"))
	{
		let argument = expression
			.trim()
			.strip_prefix("static_url(")
			.and_then(|v| v.strip_suffix(')'))
			.map(str::trim)
			.ok_or_else(|| {
				AssetBuildError::input(
					logical,
					"unsupported asset template expression; use {{ static_url('logical/path') }}",
				)
			})?;
		let name = argument
			.strip_prefix('\'')
			.and_then(|v| v.strip_suffix('\''))
			.or_else(|| argument.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
			.ok_or_else(|| {
				AssetBuildError::input(logical, "static_url requires a quoted literal logical path")
			})?;
		reinhardt_core::types::static_assets::validate_asset_path(name)?;
		return reference(logical, &format!("/{name}"), site);
	}
	reference(logical, value, site)
}

fn add_inline(
	output: &mut Vec<AssetReference>,
	references: Vec<AssetReference>,
	token: usize,
	attribute: Option<usize>,
) -> Result<(), AssetBuildError> {
	for mut reference in references {
		let inner = serde_json::from_value(reference.site)
			.map_err(|e| AssetBuildError::manifest(e.to_string()))?;
		reference.site = serde_json::to_value(Site::HtmlInline {
			token,
			attribute,
			inner: Box::new(inner),
		})
		.expect("site serializes");
		output.push(reference);
	}
	Ok(())
}

// URL collection follows srcset's tokenization: commas inside a URL (including
// data URLs) belong to that URL; trailing commas end an undescribed candidate.
fn srcset_urls(value: &str) -> Vec<(usize, usize)> {
	let bytes = value.as_bytes();
	let mut result = Vec::new();
	let mut cursor = 0;
	while cursor < bytes.len() {
		while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
		{
			cursor += 1;
		}
		let start = cursor;
		while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
			cursor += 1;
		}
		let mut end = cursor;
		while end > start && bytes[end - 1] == b',' {
			end -= 1;
		}
		if end > start {
			result.push((start, end));
		}
		if end != cursor {
			continue;
		}
		let mut parentheses = 0_u32;
		while cursor < bytes.len() {
			match bytes[cursor] {
				b'(' => parentheses += 1,
				b')' => parentheses = parentheses.saturating_sub(1),
				b',' if parentheses == 0 => {
					cursor += 1;
					break;
				}
				_ => {}
			}
			cursor += 1;
		}
	}
	result
}

pub(super) fn rewrite(
	logical: &str,
	source: &str,
	references: &[AssetReference],
	url: impl Fn(&AssetReference) -> Result<String, AssetBuildError>,
) -> Result<String, AssetBuildError> {
	let mut tokens = tokenize(source);
	type TokenEdits = BTreeMap<(usize, Option<usize>), Vec<(usize, usize, String)>>;
	let mut edits = TokenEdits::new();
	for reference in references {
		let site: Site = serde_json::from_value(reference.site.clone())
			.map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
		let value = url(reference)?;
		match site {
			Site::HtmlAttribute {
				token,
				attribute,
				candidate,
			} => {
				let Token::TagToken(tag) = &tokens[token] else {
					return Err(AssetBuildError::input(
						logical,
						"HTML attribute token changed",
					));
				};
				let text = &tag.attrs[attribute].value;
				let (start, end) = candidate
					.map(|index| srcset_urls(text)[index])
					.unwrap_or((0, text.len()));
				edits
					.entry((token, Some(attribute)))
					.or_default()
					.push((start, end, value));
			}
			Site::HtmlInline {
				token,
				attribute,
				inner,
			} => {
				edits
					.entry((token, attribute))
					.or_default()
					.push(replacement(&inner, &value)?);
			}
			_ => {
				return Err(AssetBuildError::input(
					logical,
					"non-HTML site in document rewrite",
				));
			}
		}
	}
	for ((token, attribute), mut changes) in edits {
		let text = match (&mut tokens[token], attribute) {
			(Token::TagToken(tag), Some(attribute)) => &mut tag.attrs[attribute].value,
			(Token::CharacterTokens(text), None) => text,
			_ => {
				return Err(AssetBuildError::input(
					logical,
					"HTML rewrite token changed",
				));
			}
		};
		*text = apply_edits(logical, text, &mut changes)?.into();
	}
	Ok(serialize(&tokens))
}

fn escape(value: &str, attribute: bool) -> String {
	let escaped = value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;");
	if attribute {
		escaped.replace('"', "&quot;")
	} else {
		escaped
	}
}

fn serialize(tokens: &[Token]) -> String {
	let mut output = String::new();
	let mut raw = false;
	for token in tokens {
		match token {
			Token::TagToken(tag) => {
				output.push('<');
				if tag.kind == TagKind::EndTag {
					output.push('/');
				}
				output.push_str(&tag.name);
				for attribute in &tag.attrs {
					output.push(' ');
					if let Some(prefix) = &attribute.name.prefix {
						output.push_str(prefix);
						output.push(':');
					}
					output.push_str(&attribute.name.local);
					output.push_str("=\"");
					output.push_str(&escape(&attribute.value, true));
					output.push('"');
				}
				if tag.self_closing {
					output.push('/');
				}
				output.push('>');
				raw = tag.kind == TagKind::StartTag
					&& matches!(
						tag.name.as_ref(),
						"script" | "style" | "xmp" | "iframe" | "noembed" | "noframes"
					);
			}
			Token::CharacterTokens(text) => {
				if raw {
					output.push_str(text);
				} else {
					output.push_str(&escape(text, false));
				}
			}
			Token::CommentToken(text) => {
				output.push_str("<!--");
				output.push_str(text);
				output.push_str("-->");
			}
			Token::DoctypeToken(doctype) => {
				output.push_str("<!DOCTYPE ");
				output.push_str(doctype.name.as_deref().unwrap_or("html"));
				if let Some(public) = &doctype.public_id {
					output.push_str(" PUBLIC \"");
					output.push_str(&escape(public, true));
					output.push('"');
				}
				if let Some(system) = &doctype.system_id {
					if doctype.public_id.is_none() {
						output.push_str(" SYSTEM");
					}
					output.push_str(" \"");
					output.push_str(&escape(system, true));
					output.push('"');
				}
				output.push('>');
			}
			Token::NullCharacterToken => output.push('\u{fffd}'),
			Token::EOFToken | Token::ParseError(_) => {}
		}
	}
	output
}
