//! Shared strict readers for generation and legacy manifests.

use super::model::*;
use super::validate_assignments;
use reinhardt_core::types::static_assets::AssetUrlSnapshot;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Read one supported manifest shape, rejecting duplicate JSON keys and versions.
pub fn decode_manifest(bytes: &[u8]) -> Result<DecodedAssetManifest, AssetBuildError> {
	let value: UniqueValue =
		serde_json::from_slice(bytes).map_err(|e| AssetBuildError::manifest(e.to_string()))?;
	let object = value
		.0
		.as_object()
		.ok_or_else(|| AssetBuildError::manifest("expected a JSON object"))?;
	let version = object
		.get("version")
		.map(|v| {
			v.as_str()
				.ok_or_else(|| AssetBuildError::manifest("version must be a string"))
		})
		.transpose()?;
	if version == Some("2.0") {
		let manifest: AssetManifestV2 = serde_json::from_value(value.0)
			.map_err(|e| AssetBuildError::manifest(e.to_string()))?;
		validate_manifest(&manifest)?;
		return Ok(DecodedAssetManifest::V2(manifest));
	}
	if let Some(version) = version
		&& version != "1.0"
	{
		return Err(AssetBuildError::manifest(format!(
			"unsupported version {version:?}; use an explicit supported migration"
		)));
	}
	if object.contains_key("paths") && object.contains_key("files") {
		return Err(AssetBuildError::manifest(
			"both paths and files are present; choose one manifest format",
		));
	}
	let (mapping, shape) = if let Some(paths) = object.get("paths") {
		(
			paths,
			if version.is_some() {
				LegacyManifestShape::VersionOne
			} else {
				LegacyManifestShape::Paths
			},
		)
	} else if let Some(files) = object.get("files") {
		(files, LegacyManifestShape::Files)
	} else if version.is_none() {
		(&value.0, LegacyManifestShape::Flat)
	} else {
		return Err(AssetBuildError::manifest(
			"version 1 requires paths or files",
		));
	};
	let paths: BTreeMap<String, String> = serde_json::from_value(mapping.clone())
		.map_err(|e| AssetBuildError::manifest(format!("legacy paths: {e}")))?;
	validate_assignments(
		&paths
			.iter()
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect::<Vec<_>>(),
	)?;
	Ok(DecodedAssetManifest::Legacy(LegacyAssetManifest {
		paths,
		shape,
	}))
}

/// Encode validated v2 metadata with recursively sorted object keys.
pub fn encode_manifest(manifest: &AssetManifestV2) -> Result<Vec<u8>, AssetBuildError> {
	validate_manifest(manifest)?;
	canonical_json(manifest)
}

pub(super) fn canonical_json(value: &impl serde::Serialize) -> Result<Vec<u8>, AssetBuildError> {
	fn sort(value: Value) -> Value {
		match value {
			Value::Object(map) => Value::Object(
				map.into_iter()
					.collect::<BTreeMap<_, _>>()
					.into_iter()
					.map(|(k, v)| (k, sort(v)))
					.collect(),
			),
			Value::Array(values) => Value::Array(values.into_iter().map(sort).collect()),
			other => other,
		}
	}
	let value =
		serde_json::to_value(value).map_err(|e| AssetBuildError::manifest(e.to_string()))?;
	serde_json::to_vec(&sort(value)).map_err(|e| AssetBuildError::manifest(e.to_string()))
}

/// Select an explicit manifest or an unambiguous default in a static root.
pub fn discover_manifest(root: &Path, explicit: Option<&Path>) -> Result<PathBuf, AssetBuildError> {
	if let Some(path) = explicit {
		return Ok(if path.is_absolute() {
			path.to_owned()
		} else {
			root.join(path)
		});
	}
	let current = root.join("manifest.json");
	let legacy = root.join("staticfiles.json");
	let current_exists = current
		.try_exists()
		.map_err(|e| AssetBuildError::io(&current, e))?;
	let legacy_exists = legacy
		.try_exists()
		.map_err(|e| AssetBuildError::io(&legacy, e))?;
	match (current_exists, legacy_exists) {
		(true, true) => Err(AssetBuildError::manifest(format!(
			"both {} and {} exist; select --asset-manifest explicitly or migrate the legacy publication",
			current.display(),
			legacy.display()
		))),
		(false, true) => Ok(legacy),
		_ => Ok(current),
	}
}

pub(super) fn valid_digest(value: &str) -> bool {
	value.len() == 64
		&& value
			.bytes()
			.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(super) fn validate_manifest(manifest: &AssetManifestV2) -> Result<(), AssetBuildError> {
	let invalid = |reason: String| AssetBuildError::manifest(reason);
	if manifest.version != "2.0" {
		return Err(invalid(format!(
			"unsupported version {:?}",
			manifest.version
		)));
	}
	AssetUrlSnapshot::new(
		manifest.build_id.clone(),
		"/".into(),
		manifest.paths.clone(),
	)?;
	if manifest.paths.keys().ne(manifest.assets.keys()) {
		return Err(invalid(
			"paths and assets must contain exactly the same logical names".into(),
		));
	}
	let mut processor_ids = BTreeSet::new();
	for processor in &manifest.pipeline {
		if processor.id.is_empty()
			|| processor.version.is_empty()
			|| !processor_ids.insert(&processor.id)
		{
			return Err(invalid(format!(
				"empty or duplicate processor identity {:?}",
				processor.id
			)));
		}
	}
	for (name, record) in &manifest.assets {
		let concrete_mime = record.mime.parse::<mime_guess::Mime>().is_ok_and(|mime| {
			!mime.subtype().as_str().is_empty() && mime.type_() != "*" && mime.subtype() != "*"
		});
		if !valid_digest(&record.sha256) || !concrete_mime {
			return Err(invalid(format!(
				"asset {name:?} has an invalid SHA-256 or MIME type"
			)));
		}
		if !manifest.paths[name].starts_with(&format!(
			"builds/{}/{}/",
			manifest.build_id,
			record.category.directory()
		)) {
			return Err(invalid(format!(
				"asset {name:?} does not belong to its declared category/generation"
			)));
		}
		for (kind, names) in [
			("dependencies", &record.dependencies),
			("variants", &record.variants),
		] {
			if names.windows(2).any(|pair| pair[0] >= pair[1]) {
				return Err(invalid(format!(
					"asset {name:?} {kind} must be sorted and unique"
				)));
			}
			for target in names {
				if !manifest.assets.contains_key(target) {
					return Err(invalid(format!(
						"asset {name:?} {kind} references missing {target:?}"
					)));
				}
			}
		}
		match (record.encoding, &record.parent) {
			(Some(_), Some(parent)) => {
				let canonical = manifest.assets.get(parent).ok_or_else(|| {
					invalid(format!(
						"encoded asset {name:?} has missing parent {parent:?}"
					))
				})?;
				if parent == name
					|| canonical.encoding.is_some()
					|| !canonical.variants.contains(name)
					|| canonical.mime != record.mime
					|| canonical.role != record.role
					|| canonical.category != record.category
					|| !record.variants.is_empty()
				{
					return Err(invalid(format!(
						"encoded asset {name:?} disagrees with parent {parent:?}"
					)));
				}
			}
			(None, None) => {}
			_ => {
				return Err(invalid(format!(
					"asset {name:?} requires both encoding and parent"
				)));
			}
		}
		let mut encodings = BTreeSet::new();
		for variant in &record.variants {
			let child = &manifest.assets[variant];
			if child.parent.as_deref() != Some(name)
				|| child.encoding.is_none()
				|| !encodings.insert(format!("{:?}", child.encoding))
			{
				return Err(invalid(format!(
					"asset {name:?} has an invalid or duplicate encoded variant {variant:?}"
				)));
			}
		}
	}
	for (name, entry) in &manifest.entrypoints {
		if name.is_empty() {
			return Err(invalid("entrypoint name must not be empty".into()));
		}
		for (logical, kind) in [(&entry.javascript, "JavaScript"), (&entry.wasm, "WASM")] {
			let record = manifest.assets.get(logical).ok_or_else(|| {
				invalid(format!(
					"entrypoint {name:?} lacks {kind} asset {logical:?}"
				))
			})?;
			let mime_ok = if kind == "WASM" {
				mime_is(&record.mime, "application/wasm")
			} else {
				mime_is(&record.mime, "text/javascript")
					|| mime_is(&record.mime, "application/javascript")
			};
			if record.category != AssetCategory::Pages
				|| record.role != AssetRole::Asset
				|| record.encoding.is_some()
				|| !mime_ok
			{
				return Err(invalid(format!(
					"entrypoint {name:?} has invalid {kind} asset {logical:?}"
				)));
			}
		}
		if !manifest.assets[&entry.javascript]
			.dependencies
			.contains(&entry.wasm)
		{
			return Err(invalid(format!(
				"entrypoint {name:?} JS does not declare its WASM target {:?}",
				entry.wasm
			)));
		}
		let mut styles = BTreeSet::new();
		for logical in &entry.styles {
			let record = manifest.assets.get(logical).ok_or_else(|| {
				invalid(format!("entrypoint {name:?} lacks stylesheet {logical:?}"))
			})?;
			if !mime_is(&record.mime, "text/css")
				|| record.role != AssetRole::Asset
				|| record.encoding.is_some()
				|| !styles.insert(logical)
			{
				return Err(invalid(format!(
					"entrypoint {name:?} has invalid/duplicate stylesheet {logical:?}"
				)));
			}
		}
		if let Some(logical) = &entry.document {
			let record = manifest.assets.get(logical).ok_or_else(|| {
				invalid(format!("entrypoint {name:?} lacks document {logical:?}"))
			})?;
			if record.role != AssetRole::EntryDocument
				|| !mime_is(&record.mime, "text/html")
				|| record.encoding.is_some()
			{
				return Err(invalid(format!(
					"entrypoint {name:?} has invalid document {logical:?}"
				)));
			}
		}
	}
	Ok(())
}

fn mime_is(value: &str, expected: &str) -> bool {
	value
		.split(';')
		.next()
		.unwrap_or_default()
		.trim()
		.eq_ignore_ascii_case(expected)
}

/// A JSON value reader that rejects duplicates before conversion into typed data.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = UniqueValue;
			fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.write_str("JSON with unique object keys")
			}
			fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
				Ok(UniqueValue(Value::Bool(v)))
			}
			fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
				Ok(UniqueValue(v.into()))
			}
			fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
				Ok(UniqueValue(v.into()))
			}
			fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
				serde_json::Number::from_f64(v)
					.map(|n| UniqueValue(Value::Number(n)))
					.ok_or_else(|| E::custom("non-finite JSON number"))
			}
			fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
				Ok(UniqueValue(Value::String(v.into())))
			}
			fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
				Ok(UniqueValue(Value::String(v)))
			}
			fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
				Ok(UniqueValue(Value::Null))
			}
			fn visit_seq<A: serde::de::SeqAccess<'de>>(
				self,
				mut seq: A,
			) -> Result<Self::Value, A::Error> {
				let mut values = Vec::new();
				while let Some(value) = seq.next_element::<UniqueValue>()? {
					values.push(value.0);
				}
				Ok(UniqueValue(Value::Array(values)))
			}
			fn visit_map<A: serde::de::MapAccess<'de>>(
				self,
				mut map: A,
			) -> Result<Self::Value, A::Error> {
				let mut values = serde_json::Map::new();
				while let Some((key, value)) = map.next_entry::<String, UniqueValue>()? {
					if values.insert(key.clone(), value.0).is_some() {
						return Err(serde::de::Error::custom(format!(
							"duplicate JSON key {key:?}"
						)));
					}
				}
				Ok(UniqueValue(Value::Object(values)))
			}
		}
		deserializer.deserialize_any(Visitor)
	}
}
