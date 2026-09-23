//! Producer-aware classification and collision detection.

use super::model::{AssetBuildError, AssetCategory, AssetProducer, ProcessorIdentity};
use reinhardt_core::types::static_assets::validate_asset_path;
use std::collections::BTreeMap;

const EXTENSIONS: &[(AssetCategory, &[&str])] = &[
	(
		AssetCategory::Videos,
		&[
			"mp4", "webm", "mov", "m4v", "ogv", "avi", "mkv", "mpeg", "mpg",
		],
	),
	(AssetCategory::Vectors, &["svg", "svgz", "eps", "ai"]),
	(
		AssetCategory::Images,
		&[
			"png", "jpg", "jpeg", "gif", "webp", "avif", "apng", "bmp", "tif", "tiff", "ico",
			"heic", "heif", "jxl",
		],
	),
	(AssetCategory::Css, &["css"]),
	(AssetCategory::Js, &["js", "mjs", "cjs"]),
	(
		AssetCategory::Fonts,
		&["woff", "woff2", "ttf", "otf", "eot"],
	),
	(
		AssetCategory::Audio,
		&[
			"mp3", "wav", "ogg", "oga", "flac", "m4a", "aac", "opus", "aiff", "aif",
		],
	),
];

/// Case-insensitive extension classifier with explicit custom overrides (P0).
#[derive(Debug, Clone, Default)]
pub struct AssetClassifier {
	custom: BTreeMap<String, AssetCategory>,
}

impl AssetClassifier {
	/// Register an extension without a leading dot. Pages provenance is reserved.
	pub fn register_extension(
		&mut self,
		extension: &str,
		category: AssetCategory,
	) -> Result<(), AssetBuildError> {
		let extension = extension.to_ascii_lowercase();
		if extension.is_empty()
			|| !extension
				.bytes()
				.all(|b| b.is_ascii_alphanumeric() || b == b'-')
			|| category == AssetCategory::Pages
			|| self.custom.contains_key(&extension)
		{
			return Err(AssetBuildError::input(
				extension,
				"extension mapping is invalid, duplicate, or selects reserved pages",
			));
		}
		self.custom.insert(extension, category);
		Ok(())
	}

	/// Assign a generation-relative path without changing the logical name.
	pub fn classify(
		&self,
		logical: &str,
		producer: AssetProducer,
	) -> Result<(AssetCategory, String), AssetBuildError> {
		validate_asset_path(logical)?;
		if producer == AssetProducer::Pages {
			return Ok((AssetCategory::Pages, format!("pages/{logical}")));
		}
		let lower = logical.to_ascii_lowercase();
		let mut underlying = lower.as_str();
		if let Some(base) = underlying
			.strip_suffix(".gz")
			.or_else(|| underlying.strip_suffix(".br"))
			&& self.recognizes_representation_parent(base)
		{
			underlying = base;
		}
		if let Some(base) = underlying.strip_suffix(".map") {
			underlying = base;
		}
		let extension = extension(underlying);
		let category = self.lookup(extension).unwrap_or(AssetCategory::Other);
		let rest = if let Some((first, rest)) = logical.split_once('/') {
			if is_ordinary_category(first) {
				rest
			} else {
				logical
			}
		} else {
			logical
		};
		Ok((category, format!("{}/{rest}", category.directory())))
	}

	pub(super) fn recognizes_representation_parent(&self, name: &str) -> bool {
		let name = name.strip_suffix(".map").unwrap_or(name);
		let extension = extension(name);
		self.lookup(extension).is_some()
			|| matches!(
				extension,
				"wasm" | "json" | "html" | "htm" | "txt" | "xml" | "pdf"
			)
	}

	fn lookup(&self, extension: &str) -> Option<AssetCategory> {
		self.custom.get(extension).copied().or_else(|| {
			EXTENSIONS.iter().find_map(|(category, extensions)| {
				extensions.contains(&extension).then_some(*category)
			})
		})
	}

	/// Return deterministic classification options for generation identity.
	pub fn identity(&self) -> ProcessorIdentity {
		ProcessorIdentity {
			id: "reinhardt.classifier".into(),
			version: "1".into(),
			options: serde_json::to_value(&self.custom)
				.expect("string-keyed category map serializes"),
		}
	}
}

fn extension(name: &str) -> &str {
	name.rsplit('/')
		.next()
		.unwrap_or(name)
		.rsplit_once('.')
		.map_or("", |(_, extension)| extension)
}

fn is_ordinary_category(name: &str) -> bool {
	matches!(
		name.to_ascii_lowercase().as_str(),
		"videos" | "vectors" | "images" | "css" | "js" | "fonts" | "audio" | "other"
	)
}

/// Reject duplicate logical names and portable case-folded output collisions.
pub fn validate_assignments(assignments: &[(String, String)]) -> Result<(), AssetBuildError> {
	let mut logical_names = BTreeMap::new();
	let mut output_names = BTreeMap::new();
	for (logical, path) in assignments {
		validate_asset_path(logical)?;
		validate_asset_path(path)?;
		if let Some(first) = logical_names.insert(logical, logical) {
			return Err(AssetBuildError::Collision {
				first: first.clone(),
				second: logical.clone(),
				path: logical.clone(),
			});
		}
		if let Some(first) = output_names.insert(path.to_lowercase(), logical) {
			return Err(AssetBuildError::Collision {
				first: first.clone(),
				second: logical.clone(),
				path: path.clone(),
			});
		}
	}
	Ok(())
}
