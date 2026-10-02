//! Compile entry templates into parser-free runtime instructions.

use super::{analyze, rewrite, serialize, tokenize};
use crate::staticfiles::publication::rewrite::Site;
use crate::staticfiles::publication::{
	AssetBuildError, DocumentChunk, DocumentEscaping, DocumentProgram,
};
use html5ever::tokenizer::{TagKind, Token};
use std::{cell::Cell, collections::BTreeMap};

const MARKER: &str = "REINHARDT-DOCUMENT-SLOT-";

pub(crate) fn compile(logical: &str, source: &str) -> Result<DocumentProgram, AssetBuildError> {
	if source.contains(MARKER) {
		return Err(AssetBuildError::input(
			logical,
			"template contains a reserved document marker",
		));
	}
	let tokens = tokenize(source);
	for token in &tokens {
		if let Token::TagToken(tag) = token
			&& tag.attrs.iter().any(|attr| {
				attr.name.local.as_ref() == "id"
					&& matches!(
						attr.value.as_ref(),
						"reinhardt-static-assets"
							| "reinhardt-pages-entry"
							| "reinhardt-pages-loader"
					)
			}) {
			return Err(AssetBuildError::input(
				logical,
				"template already contains a Reinhardt asset bootstrap; remove the previous injection before packaging",
			));
		}
	}
	let references = analyze(logical, source)?;
	let mut styles = Vec::new();
	let mut slots = BTreeMap::new();
	for (index, reference) in references.iter().enumerate() {
		let site: Site = serde_json::from_value(reference.site.clone())
			.map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
		let escaping = match &site {
			Site::HtmlAttribute {
				token, attribute, ..
			} => {
				if let Token::TagToken(tag) = &tokens[*token]
					&& tag.name.as_ref() == "link"
					&& tag.attrs[*attribute].name.local.as_ref() == "href"
					&& tag.attrs.iter().any(|attr| {
						attr.name.local.as_ref() == "rel"
							&& attr
								.value
								.split_ascii_whitespace()
								.any(|rel| rel.eq_ignore_ascii_case("stylesheet"))
					}) {
					if styles.contains(&reference.target) {
						return Err(AssetBuildError::input(
							logical,
							format!(
								"duplicate stylesheet link {:?}; keep one canonical link",
								reference.target
							),
						));
					}
					styles.push(reference.target.clone());
				}
				DocumentEscaping::HtmlAttribute
			}
			Site::HtmlInline {
				inner, attribute, ..
			} => match inner.as_ref() {
				Site::JavaScript { .. } => DocumentEscaping::JavaScript,
				Site::CssString { .. } | Site::CssUrl { .. } => {
					if attribute.is_some() {
						DocumentEscaping::CssAttribute
					} else {
						DocumentEscaping::Css
					}
				}
				Site::Raw { .. } if attribute.is_none() => DocumentEscaping::Comment,
				_ => {
					return Err(AssetBuildError::input(
						logical,
						"unsupported inline document reference context",
					));
				}
			},
			_ => {
				return Err(AssetBuildError::input(
					logical,
					"non-HTML document reference",
				));
			}
		};
		slots.insert(
			format!("{MARKER}{index}-END"),
			DocumentChunk::Asset {
				logical: reference.target.clone(),
				suffix: reference.suffix.clone(),
				escaping,
			},
		);
	}
	let counter = Cell::new(0);
	let rewritten = rewrite(logical, source, &references, |_| {
		let index = counter.get();
		counter.set(index + 1);
		Ok(format!("{MARKER}{index}-END"))
	})?;
	let mut tokens = tokenize(&rewritten);
	let tag_position = |name: &str, kind: TagKind| {
		tokens.iter().position(
			|token| matches!(token, Token::TagToken(tag) if tag.name.as_ref() == name && tag.kind == kind),
		)
	};
	let end = tokens
		.iter()
		.position(|token| matches!(token, Token::EOFToken))
		.unwrap_or(tokens.len());
	let first_script = tag_position("script", TagKind::StartTag).unwrap_or(end);
	let bootstrap = tag_position("head", TagKind::StartTag)
		.map(|index| index + 1)
		.or_else(|| tag_position("html", TagKind::StartTag).map(|index| index + 1))
		.unwrap_or_else(|| {
			tokens
				.iter()
				.position(|token| matches!(token, Token::DoctypeToken(_)))
				.map_or(0, |index| index + 1)
		})
		.min(first_script);
	let styles_at = tag_position("head", TagKind::EndTag)
		.or_else(|| tag_position("body", TagKind::StartTag))
		.unwrap_or(bootstrap)
		.max(bootstrap);
	let loader = tag_position("body", TagKind::EndTag)
		.unwrap_or(end)
		.max(styles_at);
	let mut injections: BTreeMap<usize, String> = BTreeMap::new();
	for (position, key, chunk) in [
		(bootstrap, "bootstrap", DocumentChunk::Bootstrap),
		(styles_at, "styles", DocumentChunk::Styles),
		(loader, "loader", DocumentChunk::Loader),
	] {
		let marker = format!("{MARKER}{key}-END");
		injections.entry(position).or_default().push_str(&marker);
		slots.insert(marker, chunk);
	}
	for (position, marker) in injections.into_iter().rev() {
		tokens.insert(position, Token::CharacterTokens(marker.into()));
	}
	let rendered = serialize(&tokens);
	let mut remaining = rendered.as_str();
	let mut chunks = Vec::new();
	while let Some(index) = remaining.find(MARKER) {
		if index > 0 {
			chunks.push(DocumentChunk::Text {
				value: remaining[..index].into(),
			});
		}
		remaining = &remaining[index..];
		let length = remaining
			.find("-END")
			.map(|index| index + 4)
			.ok_or_else(|| AssetBuildError::input(logical, "invalid render marker"))?;
		let chunk = slots.remove(&remaining[..length]).ok_or_else(|| {
			AssetBuildError::input(logical, "repeated or unrecognized document slot")
		})?;
		chunks.push(chunk);
		remaining = &remaining[length..];
	}
	if !remaining.is_empty() {
		chunks.push(DocumentChunk::Text {
			value: remaining.into(),
		});
	}
	if !slots.is_empty() {
		return Err(AssetBuildError::input(
			logical,
			"document serialization dropped a reference slot",
		));
	}
	Ok(DocumentProgram {
		version: 1,
		chunks,
		styles,
	})
}
