//! Syntax-aware local references; no global filename replacement.

mod css;
mod html;
mod javascript;
mod manifest;

pub(super) use html::document::compile as compile_document;

use super::model::*;
use super::pipeline::*;
use reinhardt_core::types::static_assets::{encode_asset_path, validate_asset_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Site {
	JavaScript {
		start: usize,
		end: usize,
	},
	CssString {
		start: usize,
		end: usize,
	},
	CssUrl {
		start: usize,
		end: usize,
	},
	Raw {
		start: usize,
		end: usize,
	},
	HtmlAttribute {
		token: usize,
		attribute: usize,
		candidate: Option<usize>,
	},
	HtmlInline {
		token: usize,
		attribute: Option<usize>,
		inner: Box<Site>,
	},
}

pub(super) struct BuiltinProcessor;

impl AssetProcessor for BuiltinProcessor {
	fn identity(&self) -> ProcessorIdentity {
		ProcessorIdentity {
			id: "reinhardt.references".into(),
			version: "2".into(),
			options: serde_json::json!({
				"entry_document_contract":1,
				"inline_script_contract":2,
				"pages_loader_contract":1,
				"web_manifest_contract":1,
			}),
		}
	}
	fn matches(&self, asset: &PreparedAsset) -> bool {
		if is_generated_pages_loader(asset.logical_path()) {
			return false;
		}
		matches!(
			asset.mime().split(';').next().unwrap_or_default(),
			"text/javascript"
				| "application/javascript"
				| "text/css" | "text/html"
				| "application/manifest+json"
		)
	}
	fn prepare(&self, _: &PreparedAsset) -> Result<Vec<AssetInput>, AssetBuildError> {
		Ok(Vec::new())
	}
	fn analyze(&self, asset: &PreparedAsset) -> Result<Vec<AssetReference>, AssetBuildError> {
		if is_generated_pages_loader(asset.logical_path()) {
			return Ok(Vec::new());
		}
		analyze_asset_references(asset.logical_path(), asset.mime(), &asset.read()?)
	}

	fn rewrite(
		&self,
		asset: &AnalyzedAsset,
		paths: &BTreeMap<String, String>,
	) -> Result<RewriteOutput, AssetBuildError> {
		let bytes = asset.asset.read()?;
		let is_web_manifest =
			asset.asset.mime().split(';').next() == Some("application/manifest+json");
		if asset.references.is_empty() && !is_web_manifest {
			return Ok(RewriteOutput {
				bytes,
				source_map: None,
			});
		}
		let source = std::str::from_utf8(&bytes)
			.map_err(|e| AssetBuildError::input(asset.asset.logical_path(), e.to_string()))?;
		if asset.asset.role() == AssetRole::EntryDocument {
			let logical = asset.asset.logical_path();
			let already_logical = analyze_asset_references(logical, asset.asset.mime(), &bytes)
				.is_ok_and(|references| {
					references.len() == asset.references.len()
						&& references
							.iter()
							.zip(&asset.references)
							.all(|(original, resolved)| {
								original.target == resolved.target
									&& original.suffix == resolved.suffix
							})
				});
			if already_logical {
				return Ok(RewriteOutput {
					bytes,
					source_map: None,
				});
			}
			// Input aliases are not a second runtime manifest. Canonicalize those references
			// into the document's logical namespace before hashing the render template.
			let rewritten = html::rewrite(logical, source, &asset.references, |reference| {
				Ok(format!(
					"{}{}",
					relative_asset_url(logical, &reference.target)?,
					reference.suffix
				))
			})?;
			return Ok(RewriteOutput {
				bytes: rewritten.into_bytes(),
				source_map: None,
			});
		}
		let location = &paths[asset.asset.logical_path()];
		let url = |reference: &AssetReference| -> Result<String, AssetBuildError> {
			Ok(format!(
				"{}{}",
				relative_asset_url(location, &paths[&reference.target])?,
				reference.suffix
			))
		};
		let rewritten = if is_web_manifest {
			// Web manifests are rewritten separately because URL-valued members are
			// JSON strings rather than CSS, JavaScript, or HTML reference sites.
			manifest::rewrite(
				asset.asset.logical_path(),
				location,
				source,
				&asset.references,
				&url,
			)?
		} else if asset.asset.mime().split(';').next() == Some("text/html") {
			html::rewrite(asset.asset.logical_path(), source, &asset.references, url)?
		} else {
			let mut edits = Vec::new();
			for reference in &asset.references {
				let site: Site = serde_json::from_value(reference.site.clone()).map_err(|e| {
					AssetBuildError::input(asset.asset.logical_path(), e.to_string())
				})?;
				let value = url(reference)?;
				let (start, end, replacement) = replacement(&site, &value)?;
				edits.push((start, end, replacement));
			}
			apply_edits(asset.asset.logical_path(), source, &mut edits)?
		};
		Ok(RewriteOutput {
			bytes: rewritten.into_bytes(),
			source_map: None,
		})
	}
}

fn is_generated_pages_loader(logical: &str) -> bool {
	let reserved = super::pipeline::PAGES_LOADER_LOGICAL;
	logical == reserved
		|| logical
			.strip_suffix(reserved)
			.is_some_and(|prefix| prefix.ends_with('/'))
}

/// Discover built-in JS, CSS, or HTML dependencies in a specified input namespace (P0).
/// This performs syntax analysis only; the complete pipeline validates target existence.
pub fn analyze_asset_references(
	logical: &str,
	mime: &str,
	bytes: &[u8],
) -> Result<Vec<AssetReference>, AssetBuildError> {
	let mime = mime
		.parse::<mime_guess::Mime>()
		.map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
	if !matches!(
		mime.essence_str(),
		"text/javascript"
			| "application/javascript"
			| "text/css"
			| "text/html"
			| "application/manifest+json"
	) {
		return Ok(Vec::new());
	}
	if mime
		.get_param("charset")
		.is_some_and(|value| !value.as_str().eq_ignore_ascii_case("utf-8"))
	{
		return Err(AssetBuildError::input(
			logical,
			"built-in reference processing requires a UTF-8 charset; convert the input or register a custom processor",
		));
	}
	let source = std::str::from_utf8(bytes).map_err(|e| {
		AssetBuildError::input(logical, format!("reference processing requires UTF-8: {e}"))
	})?;
	match mime.essence_str() {
		"text/javascript" | "application/javascript" => javascript::analyze(logical, source),
		"text/css" => css::analyze(logical, source),
		"text/html" => html::analyze(logical, source),
		"application/manifest+json" => manifest::analyze(logical, source),
		_ => Ok(Vec::new()),
	}
}

pub(super) fn reference(
	logical: &str,
	value: &str,
	site: Site,
) -> Result<Option<AssetReference>, AssetBuildError> {
	Ok(
		resolve_asset_reference(logical, value)?.map(|(target, suffix)| AssetReference {
			target,
			suffix,
			site: serde_json::to_value(site).expect("reference sites contain serializable data"),
		}),
	)
}

/// Resolve a local URL in a logical source namespace, preserving query/fragment (P0).
/// External, data, blob, protocol-relative, and fragment-only URLs return `None`.
pub fn resolve_asset_reference(
	source: &str,
	reference: &str,
) -> Result<Option<(String, String)>, AssetBuildError> {
	let reference = reference.trim();
	if reference.is_empty()
		|| reference.starts_with(['#'])
		|| reference.starts_with("//")
		|| external(reference)
	{
		return Ok(None);
	}
	let (pathname, suffix) = reference.find(['?', '#']).map_or((reference, ""), |index| {
		(&reference[..index], &reference[index..])
	});
	let mut parts: Vec<String> = if pathname.starts_with('/') {
		Vec::new()
	} else {
		source.rsplit_once('/').map_or(Vec::new(), |(parent, _)| {
			parent.split('/').map(str::to_owned).collect()
		})
	};
	if pathname.is_empty() {
		return Ok(Some((source.into(), suffix.into())));
	}
	for segment in pathname.strip_prefix('/').unwrap_or(pathname).split('/') {
		let decoded = percent_encoding::percent_decode_str(segment)
			.decode_utf8()
			.map_err(|e| AssetBuildError::input(source, e.to_string()))?;
		if decoded.contains(['/', '\\']) {
			return Err(AssetBuildError::input(
				source,
				format!("encoded separator in asset reference {reference:?}"),
			));
		}
		match decoded.as_ref() {
			"." => {}
			".." => {
				if parts.pop().is_none() {
					return Err(AssetBuildError::input(
						source,
						format!("asset reference {reference:?} escapes its source root"),
					));
				}
			}
			"" => {
				return Err(AssetBuildError::input(
					source,
					format!("empty segment in asset reference {reference:?}"),
				));
			}
			part => parts.push(part.into()),
		}
	}
	let target = parts.join("/");
	validate_asset_path(&target)?;
	Ok(Some((target, suffix.into())))
}

pub(super) fn external(value: &str) -> bool {
	value.split_once(':').is_some_and(|(scheme, _)| {
		!scheme.is_empty()
			&& scheme.as_bytes()[0].is_ascii_alphabetic()
			&& scheme
				.bytes()
				.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
	})
}

/// Construct a relocatable URL between decoded generation-relative asset paths (P0).
pub fn relative_asset_url(source: &str, target: &str) -> Result<String, AssetBuildError> {
	validate_asset_path(source)?;
	validate_asset_path(target)?;
	let mut source_parts: Vec<_> = source.split('/').collect();
	source_parts.pop();
	let target_parts: Vec<_> = target.split('/').collect();
	let common = source_parts
		.iter()
		.zip(&target_parts)
		.take_while(|(a, b)| a == b)
		.count();
	let upwards = source_parts.len() - common;
	let prefix = if upwards == 0 {
		"./".into()
	} else {
		"../".repeat(upwards)
	};
	Ok(format!(
		"{prefix}{}",
		encode_asset_path(&target_parts[common..].join("/"))?
	))
}

pub(super) fn replacement(
	site: &Site,
	value: &str,
) -> Result<(usize, usize, String), AssetBuildError> {
	match *site {
		Site::JavaScript { start, end } => Ok((
			start,
			end,
			serde_json::to_string(value)
				.expect("string serializes")
				.replace('<', "\\u003c")
				.replace('>', "\\u003e")
				.replace('&', "\\u0026"),
		)),
		Site::CssString { start, end } | Site::CssUrl { start, end } => {
			let mut quoted = String::new();
			cssparser::serialize_string(value, &mut quoted)
				.expect("writing to String is infallible");
			Ok((
				start,
				end,
				if matches!(site, Site::CssUrl { .. }) {
					format!("url({quoted})")
				} else {
					quoted
				},
			))
		}
		Site::Raw { start, end } => Ok((start, end, value.into())),
		Site::HtmlAttribute { .. } | Site::HtmlInline { .. } => Err(AssetBuildError::manifest(
			"HTML references require token rewriting",
		)),
	}
}

pub(super) fn apply_edits(
	logical: &str,
	source: &str,
	edits: &mut [(usize, usize, String)],
) -> Result<String, AssetBuildError> {
	edits.sort_by_key(|(start, end, _)| (*start, *end));
	let mut output = String::with_capacity(source.len());
	let mut cursor = 0;
	for (start, end, replacement) in edits {
		if *start < cursor
			|| *end < *start
			|| !source.is_char_boundary(*start)
			|| !source.is_char_boundary(*end)
		{
			return Err(AssetBuildError::input(
				logical,
				"reference rewrite sites overlap or are outside the source",
			));
		}
		output.push_str(&source[cursor..*start]);
		output.push_str(replacement);
		cursor = *end;
	}
	output.push_str(&source[cursor..]);
	Ok(output)
}

pub(super) fn map_reference(
	logical: &str,
	comment: &str,
	offset: usize,
) -> Result<Option<AssetReference>, AssetBuildError> {
	let Some((_, value)) = comment
		.trim_start()
		.strip_prefix('#')
		.or_else(|| comment.trim_start().strip_prefix('@'))
		.and_then(|c| c.trim_start().split_once("sourceMappingURL="))
	else {
		return Ok(None);
	};
	let value = value.trim();
	if value.is_empty() {
		return Err(AssetBuildError::input(logical, "empty sourceMappingURL"));
	}
	if external(value) || value.starts_with("//") {
		return Err(AssetBuildError::input(
			logical,
			"external or inline sourceMappingURL cannot be verified; supply a local map or a processor with explicit map support",
		));
	}
	let start = offset + comment.rfind(value).expect("value is a comment suffix");
	reference(
		logical,
		value,
		Site::Raw {
			start,
			end: start + value.len(),
		},
	)
}
