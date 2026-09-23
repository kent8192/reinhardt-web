//! Remap generated UTF-16 positions through the exact syntax edits.

use super::super::manifest::canonical_json;
use super::super::model::*;
use super::super::pipeline::{AnalyzedAsset, InputNamespace, PreparedAsset};
use super::super::rewrite::{
	Site, apply_edits, external, relative_asset_url, replacement, resolve_asset_reference,
};
use std::collections::BTreeMap;

pub(crate) fn rewrite(
	asset: &AnalyzedAsset,
	output: &[u8],
	paths: &BTreeMap<String, String>,
	inputs: &BTreeMap<String, PreparedAsset>,
	processor: &str,
	namespace: &InputNamespace,
) -> Result<Option<(String, Vec<u8>)>, AssetBuildError> {
	let maps: Vec<_> = asset
		.references
		.iter()
		.filter(|r| inputs[&r.target].role == AssetRole::SourceMap)
		.collect();
	if maps.is_empty() || asset.asset.role == AssetRole::EntryDocument {
		return Ok(None);
	}
	let logical = asset.asset.logical_path();
	if maps.len() != 1 {
		return Err(AssetBuildError::input(
			logical,
			"multiple source maps require a custom processor with explicit map output",
		));
	}
	let map_name = &maps[0].target;
	let original = asset.asset.read()?;
	if processor != "reinhardt.references" && original != output {
		return Err(AssetBuildError::Processor {
			processor: processor.into(),
			asset: logical.into(),
			reason: format!(
				"changed mapped bytes without updated positions for {map_name}; return RewriteOutput.source_map"
			),
		});
	}
	let source = std::str::from_utf8(&original)
		.map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
	let output =
		std::str::from_utf8(output).map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
	let mut edits = Vec::new();
	if processor == "reinhardt.references" {
		for reference in &asset.references {
			let site: Site = serde_json::from_value(reference.site.clone())
				.map_err(|e| AssetBuildError::input(logical, e.to_string()))?;
			let url = format!(
				"{}{}",
				relative_asset_url(&paths[logical], &paths[&reference.target])?,
				reference.suffix
			);
			edits.push(replacement(&site, &url).map_err(|_| {
				AssetBuildError::input(
					logical,
					"inline HTML source maps require a custom processor that supplies corrected positions",
				)
			})?);
		}
		if apply_edits(logical, source, &mut edits)? != output {
			return Err(AssetBuildError::input(
				logical,
				"source-map edits do not describe the rewritten bytes",
			));
		}
	}
	let map_bytes = inputs[map_name].read()?;
	let map = sourcemap::SourceMap::from_slice(&map_bytes).map_err(|e| {
		AssetBuildError::input(
			map_name,
			format!("unsupported or invalid flat source map: {e}"),
		)
	})?;
	let mut value: serde_json::Value = serde_json::from_slice(&map_bytes)
		.map_err(|e| AssetBuildError::input(map_name, e.to_string()))?;
	let mut builder = sourcemap::SourceMapBuilder::new(None);
	for source in map.sources() {
		builder.add_source(source);
	}
	for name in map.names() {
		builder.add_name(name);
	}
	for token in map.tokens() {
		let offset = byte_offset(source, token.get_dst()).ok_or_else(|| {
			AssetBuildError::input(
				map_name,
				format!(
					"mapping {:?} is outside generated source {logical}",
					token.get_dst()
				),
			)
		})?;
		let mut adjusted = offset as i64;
		for (start, end, replacement) in &edits {
			if offset < *start {
				break;
			}
			if offset < *end {
				adjusted -= (offset - start) as i64;
				break;
			}
			adjusted += replacement.len() as i64 - (end - start) as i64;
		}
		let (line, column) = position(output, adjusted as usize).ok_or_else(|| {
			AssetBuildError::input(
				map_name,
				"rewritten source-map position is not a UTF-8 boundary",
			)
		})?;
		let raw = token.get_raw_token();
		builder.add_raw(
			line,
			column,
			raw.src_line,
			raw.src_col,
			(raw.src_id != u32::MAX).then_some(raw.src_id),
			(raw.name_id != u32::MAX).then_some(raw.name_id),
			raw.is_range,
		);
	}
	let mut mapped = Vec::new();
	builder
		.into_sourcemap()
		.to_writer(&mut mapped)
		.map_err(|e| AssetBuildError::input(map_name, e.to_string()))?;
	let generated: serde_json::Value = serde_json::from_slice(&mapped)
		.map_err(|e| AssetBuildError::input(map_name, e.to_string()))?;
	value["mappings"] = generated["mappings"].clone();
	value["file"] = relative_asset_url(&paths[map_name], &paths[logical])?.into();
	let root = value
		.get("sourceRoot")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_owned();
	if !external(&root) && !root.starts_with("//") {
		let contents = value
			.get("sourcesContent")
			.and_then(|v| v.as_array())
			.cloned()
			.unwrap_or_default();
		if let Some(sources) = value.get_mut("sources").and_then(|v| v.as_array_mut()) {
			for (index, source) in sources.iter_mut().enumerate() {
				let name = source.as_str().ok_or_else(|| {
					AssetBuildError::input(map_name, "source map source is not a string")
				})?;
				let reference = if root.is_empty() || external(name) || name.starts_with('/') {
					name.to_owned()
				} else {
					format!("{}/{name}", root.trim_end_matches('/'))
				};
				let resolved = match resolve_asset_reference(namespace.base(map_name), &reference) {
					Ok(value) => value,
					Err(_) if contents.get(index).is_some_and(|v| v.is_string()) => {
						// Embedded sources are debugger identities, not filesystem reads.
						*source = reference.into();
						continue;
					}
					Err(error) => return Err(error),
				};
				if let Some((target, suffix)) = resolved {
					if let Some(path) = paths.get(namespace.logical(&target)) {
						*source =
							format!("{}{}", relative_asset_url(&paths[map_name], path)?, suffix)
								.into();
					} else if contents.get(index).is_some_and(|v| v.is_string()) {
						*source = reference.into();
					} else {
						return Err(AssetBuildError::input(
							map_name,
							format!(
								"missing original source {target:?}; include it or embed sourcesContent"
							),
						));
					}
				}
			}
		}
		value
			.as_object_mut()
			.expect("source map is an object")
			.remove("sourceRoot");
	}
	Ok(Some((map_name.clone(), canonical_json(&value)?)))
}

fn byte_offset(source: &str, (line, column): (u32, u32)) -> Option<usize> {
	let start = if line == 0 {
		0
	} else {
		source.match_indices('\n').nth(line as usize - 1)?.0 + 1
	};
	let mut units = 0;
	for (offset, c) in source[start..].char_indices() {
		if units == column {
			return Some(start + offset);
		}
		if c == '\n' {
			return None;
		}
		units += c.len_utf16() as u32;
	}
	(units == column).then_some(source.len())
}

fn position(source: &str, offset: usize) -> Option<(u32, u32)> {
	let prefix = source.get(..offset)?;
	let line = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
	let column = prefix.rsplit('\n').next()?.encode_utf16().count() as u32;
	Some((line, column))
}
