//! Immutable static asset URL projections (P2: identical native/WASM semantics).
//!
//! A projection comes from one validated publication manifest. It contains no
//! filesystem operations and never follows a newer manifest during a page's life.

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// Encode every reserved URL delimiter within a filename, including literal `%`.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
	.add(b' ')
	.add(b'!')
	.add(b'"')
	.add(b'#')
	.add(b'$')
	.add(b'%')
	.add(b'&')
	.add(b'\'')
	.add(b'(')
	.add(b')')
	.add(b'*')
	.add(b'+')
	.add(b',')
	.add(b'/')
	.add(b':')
	.add(b';')
	.add(b'<')
	.add(b'=')
	.add(b'>')
	.add(b'?')
	.add(b'@')
	.add(b'[')
	.add(b'\\')
	.add(b']')
	.add(b'^')
	.add(b'`')
	.add(b'{')
	.add(b'|')
	.add(b'}');

/// Failure to validate or resolve an immutable asset URL projection (P2).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AssetUrlError {
	/// A generation ID must be a full lowercase SHA-256 hex digest.
	#[error("asset build ID must contain 64 lowercase hexadecimal characters")]
	InvalidBuildId,
	/// A prefix must be a safe root-relative path or absolute HTTP(S) URL.
	#[error(
		"STATIC_URL must be a root-relative path or HTTP(S) URL without credentials, query, fragment, or traversal"
	)]
	InvalidPrefix,
	/// A path is unsafe or does not belong to the selected generation.
	#[error("invalid published asset path: {path}")]
	InvalidPath {
		/// Rejected path.
		path: String,
	},
	/// The selected generation does not declare this logical name.
	#[error(
		"unknown logical asset {logical:?}; rebuild the publication or use a declared logical name"
	)]
	UnknownLogicalPath {
		/// Requested logical name.
		logical: String,
	},
	/// The public JSON projection is malformed.
	#[error("invalid static asset projection: {reason}")]
	InvalidProjection {
		/// Validation diagnostic.
		reason: String,
	},
}

/// A page's immutable logical-to-published URL mapping (P2).
///
/// Published paths are decoded relative filenames, including `builds/<id>/`.
/// Resolution percent-encodes their segments exactly once. Unknown logical names
/// are errors; this type never falls back to an unhashed path.
///
/// ```
/// use reinhardt_core::types::static_assets::AssetUrlSnapshot;
/// use std::collections::BTreeMap;
/// let id = "a".repeat(64);
/// let urls = AssetUrlSnapshot::new(id.clone(), "/console/static/".into(),
///     BTreeMap::from([("app.js".into(), format!("builds/{id}/pages/app.js"))]))?;
/// assert!(urls.resolve("app.js")?.starts_with("/console/static/builds/"));
/// assert!(urls.resolve("missing.js").is_err());
/// # Ok::<(), reinhardt_core::types::static_assets::AssetUrlError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetUrlSnapshot {
	data: Projection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Projection {
	build_id: String,
	static_url: String,
	#[serde(deserialize_with = "unique_paths")]
	paths: BTreeMap<String, String>,
}

fn unique_paths<'de, D: serde::Deserializer<'de>>(
	deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
	struct Paths;
	impl<'de> serde::de::Visitor<'de> for Paths {
		type Value = BTreeMap<String, String>;
		fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			formatter.write_str("a map of unique logical asset names to paths")
		}
		fn visit_map<A: serde::de::MapAccess<'de>>(
			self,
			mut entries: A,
		) -> Result<Self::Value, A::Error> {
			let mut paths = BTreeMap::new();
			while let Some((name, path)) = entries.next_entry::<String, String>()? {
				if paths.insert(name.clone(), path).is_some() {
					return Err(serde::de::Error::custom(format!(
						"duplicate logical asset {name:?}"
					)));
				}
			}
			Ok(paths)
		}
	}
	deserializer.deserialize_map(Paths)
}

impl AssetUrlSnapshot {
	/// Validate the complete public projection before it becomes usable.
	pub fn new(
		build_id: String,
		static_url: String,
		paths: BTreeMap<String, String>,
	) -> Result<Self, AssetUrlError> {
		if build_id.len() != 64
			|| !build_id
				.bytes()
				.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
		{
			return Err(AssetUrlError::InvalidBuildId);
		}
		let static_url = normalize_static_url(&static_url)?;
		let generation_prefix = format!("builds/{build_id}/");
		let mut assigned = BTreeSet::new();
		for (name, path) in &paths {
			validate_asset_path(name)?;
			validate_asset_path(path)?;
			if !path.starts_with(&generation_prefix) || !assigned.insert(path.to_lowercase()) {
				return Err(AssetUrlError::InvalidPath { path: path.clone() });
			}
		}
		Ok(Self {
			data: Projection {
				build_id,
				static_url,
				paths,
			},
		})
	}

	/// Resolve an exact logical name within this generation.
	pub fn resolve(&self, logical: &str) -> Result<String, AssetUrlError> {
		let path =
			self.data
				.paths
				.get(logical)
				.ok_or_else(|| AssetUrlError::UnknownLogicalPath {
					logical: logical.into(),
				})?;
		Ok(format!(
			"{}{}",
			self.data.static_url,
			encode_asset_path(path)?
		))
	}

	/// Return the generation to which this projection is bound.
	pub fn build_id(&self) -> &str {
		&self.data.build_id
	}

	/// Return the validated, slash-terminated public prefix.
	pub fn static_url(&self) -> &str {
		&self.data.static_url
	}

	/// Return the immutable decoded publication paths.
	pub fn paths(&self) -> &BTreeMap<String, String> {
		&self.data.paths
	}

	/// Serialize the public projection. HTML producers must also escape its context.
	pub fn to_json(&self) -> Result<String, AssetUrlError> {
		serde_json::to_string(&self.data).map_err(|e| AssetUrlError::InvalidProjection {
			reason: e.to_string(),
		})
	}

	/// Decode and revalidate a projection, including duplicate logical names.
	pub fn from_json(json: &str) -> Result<Self, AssetUrlError> {
		let data: Projection =
			serde_json::from_str(json).map_err(|e| AssetUrlError::InvalidProjection {
				reason: e.to_string(),
			})?;
		Self::new(data.build_id, data.static_url, data.paths)
	}
}

/// Validate a decoded, portable relative asset path (P2).
///
/// Literal percent signs are filenames, not an additional URL-decoding layer.
pub fn validate_asset_path(path: &str) -> Result<(), AssetUrlError> {
	if path.is_empty()
		|| path
			.chars()
			.any(|c| c.is_control() || c == '\\' || c == ':')
		|| path.split('/').any(|part| {
			part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
		}) {
		return Err(AssetUrlError::InvalidPath { path: path.into() });
	}
	Ok(())
}

/// Percent-encode each segment of a validated decoded relative filename (P2).
pub fn encode_asset_path(path: &str) -> Result<String, AssetUrlError> {
	validate_asset_path(path)?;
	Ok(path
		.split('/')
		.map(|part| utf8_percent_encode(part, PATH_SEGMENT).to_string())
		.collect::<Vec<_>>()
		.join("/"))
}

/// Validate and normalize the public static prefix without changing its origin (P2).
pub fn normalize_static_url(prefix: &str) -> Result<String, AssetUrlError> {
	if prefix
		.chars()
		.any(|c| c.is_control() || c.is_whitespace() || c == '\\')
		|| prefix.contains(['?', '#'])
		|| prefix.starts_with("//")
	{
		return Err(AssetUrlError::InvalidPrefix);
	}
	let absolute = !prefix.starts_with('/');
	let raw_path = if absolute {
		let (_, rest) = prefix
			.split_once("://")
			.ok_or(AssetUrlError::InvalidPrefix)?;
		rest.find('/').map_or("/", |index| &rest[index..])
	} else {
		prefix
	};
	let without_slashes = raw_path
		.strip_prefix('/')
		.ok_or(AssetUrlError::InvalidPrefix)?
		.strip_suffix('/')
		.unwrap_or_else(|| &raw_path[1..]);
	if !without_slashes.is_empty() {
		for part in without_slashes.split('/') {
			let decoded = percent_decode_str(part)
				.decode_utf8()
				.map_err(|_| AssetUrlError::InvalidPrefix)?;
			if decoded.contains('/') || validate_asset_path(&decoded).is_err() {
				return Err(AssetUrlError::InvalidPrefix);
			}
		}
	}
	let url = if absolute {
		url::Url::parse(prefix)
	} else {
		url::Url::parse(&format!("https://static.invalid{prefix}"))
	}
	.map_err(|_| AssetUrlError::InvalidPrefix)?;
	if !matches!(url.scheme(), "http" | "https")
		|| url.host_str().is_none()
		|| !url.username().is_empty()
		|| url.password().is_some()
	{
		return Err(AssetUrlError::InvalidPrefix);
	}
	let mut normalized = if absolute {
		url.to_string()
	} else {
		url.path().into()
	};
	if !normalized.ends_with('/') {
		normalized.push('/');
	}
	Ok(normalized)
}
