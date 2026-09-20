//! Versioned publication data; parsing metadata is not filesystem readiness.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Actionable failures from asset preparation, publication, or verification (P0).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetBuildError {
	/// Input or logical-name validation failed.
	#[error("invalid asset {asset:?}: {reason}")]
	Input {
		/// Source/logical asset name.
		asset: String,
		/// How the input violated the contract.
		reason: String,
	},
	/// Two logical sources would claim one name or path.
	#[error(
		"asset collision between {first:?} and {second:?} at {path:?}; assign distinct logical namespaces"
	)]
	Collision {
		/// First source.
		first: String,
		/// Second source.
		second: String,
		/// Conflicting output or logical path.
		path: String,
	},
	/// Reference discovery or relocation could not be completed.
	#[error("asset reference in {asset:?} to {target:?}: {reason}")]
	Reference {
		/// Referencing logical asset.
		asset: String,
		/// Requested dependency.
		target: String,
		/// Missing/unsupported reference diagnostic.
		reason: String,
	},
	/// A registered processor failed or violated its declaration.
	#[error("asset processor {processor:?} failed for {asset:?}: {reason}")]
	Processor {
		/// Stable processor ID.
		processor: String,
		/// Logical source.
		asset: String,
		/// Failure details.
		reason: String,
	},
	/// Manifest metadata is invalid or ambiguous.
	#[error("invalid static asset manifest: {reason}")]
	Manifest {
		/// Parse or consistency failure.
		reason: String,
	},
	/// Filesystem work failed at a specific path.
	#[error("static asset I/O at {path}: {source}")]
	Io {
		/// Failed path.
		path: PathBuf,
		/// Operating-system failure.
		#[source]
		source: std::io::Error,
	},
	/// The deployment selected a different generation than expected.
	#[error("stale static publication: expected {expected}, observed {observed}")]
	StaleGeneration {
		/// Required generation.
		expected: String,
		/// Manifest generation.
		observed: String,
	},
	/// Development and production publications cannot share one root.
	#[error(
		"static publication mode mismatch: expected {expected:?}, observed {observed:?}; use a distinct static root"
	)]
	ModeMismatch {
		/// Requested serving/publication mode.
		expected: AssetMode,
		/// Existing generation mode.
		observed: AssetMode,
	},
	/// Immutable output or atomic activation could not be guaranteed.
	#[error("static publication conflict: {reason}")]
	PublicationConflict {
		/// Existing output or activation diagnostic.
		reason: String,
	},
	/// URL projection validation failed.
	#[error(transparent)]
	Url(#[from] reinhardt_core::types::static_assets::AssetUrlError),
}

impl AssetBuildError {
	pub(super) fn manifest(reason: impl Into<String>) -> Self {
		Self::Manifest {
			reason: reason.into(),
		}
	}
	pub(super) fn input(asset: impl Into<String>, reason: impl Into<String>) -> Self {
		Self::Input {
			asset: asset.into(),
			reason: reason.into(),
		}
	}
	pub(super) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
		Self::Io {
			path: path.into(),
			source,
		}
	}
}

/// Explicit publication/serving mode, independent of compiler optimization (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetMode {
	/// Immutable generation assets and revalidated entry documents.
	Production,
	/// Complete generations served without caching.
	Development,
}

/// Fixed output directories; custom extensions select an existing category (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetCategory {
	/// Complete wasm-bindgen output sets.
	Pages,
	/// Video files.
	Videos,
	/// Vector graphics.
	Vectors,
	/// Raster images.
	Images,
	/// Stylesheets and their companions.
	Css,
	/// Ordinary JavaScript and its companions.
	Js,
	/// Web fonts.
	Fonts,
	/// Audio files.
	Audio,
	/// All remaining static formats.
	Other,
}

impl AssetCategory {
	/// Return the directory name used in publication paths.
	pub const fn directory(self) -> &'static str {
		match self {
			Self::Pages => "pages",
			Self::Videos => "videos",
			Self::Vectors => "vectors",
			Self::Images => "images",
			Self::Css => "css",
			Self::Js => "js",
			Self::Fonts => "fonts",
			Self::Audio => "audio",
			Self::Other => "other",
		}
	}
}

/// Semantic role determining validation and response rendering (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetRole {
	/// Ordinary immutable bytes.
	Asset,
	/// A map describing transformed source positions.
	SourceMap,
	/// A stored HTML template rendered with one selected generation.
	EntryDocument,
}

/// HTTP content encoding for a separately published representation (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetEncoding {
	/// Gzip representation.
	#[serde(rename = "gzip")]
	Gzip,
	/// Brotli representation.
	#[serde(rename = "br")]
	Brotli,
}

/// Provenance that controls placement ahead of the file extension (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetProducer {
	/// Ordinary collected, virtual, or vendor content.
	Static,
	/// A file belonging to a complete Pages build output tree.
	Pages,
}

/// A published asset's metadata; its physical path appears only in `paths` (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
	/// Placement category.
	pub category: AssetCategory,
	/// Rendering or source-map role.
	pub role: AssetRole,
	/// MIME of the decoded representation.
	pub mime: String,
	/// Final representation byte length.
	pub size: u64,
	/// Full SHA-256 of the final representation bytes.
	pub sha256: String,
	/// Sorted logical dependency names.
	pub dependencies: Vec<String>,
	/// Sorted separately declared encoded representations.
	pub variants: Vec<String>,
	/// Representation encoding; absent for canonical bytes.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub encoding: Option<AssetEncoding>,
	/// Canonical logical parent of an encoded representation.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<String>,
}

/// Explicit relationships used by the Pages loader, without filename scans (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PagesEntrypoint {
	/// Logical JS module entry.
	pub javascript: String,
	/// Logical WASM module initialized by the entry.
	pub wasm: String,
	/// Ordered logical stylesheet names.
	pub styles: Vec<String>,
	/// Optional logical entry-document template.
	pub document: Option<String>,
}

/// Stable processor/classifier identity included in generation hashing (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessorIdentity {
	/// Stable unique identifier.
	pub id: String,
	/// Algorithm/contract version.
	pub version: String,
	/// Public deterministic options; never credentials or private source paths.
	pub options: serde_json::Value,
}

/// Version 2 manifest metadata. Filesystem verification is a separate step (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetManifestV2 {
	/// Explicit schema version, always `2.0` for this contract.
	pub version: String,
	/// Explicit production/development mode.
	pub mode: AssetMode,
	/// Full generation inventory digest.
	pub build_id: String,
	/// Logical-to-static-root-relative published paths.
	pub paths: BTreeMap<String, String>,
	/// One metadata record for every mapped logical name.
	pub assets: BTreeMap<String, AssetRecord>,
	/// Named Pages JS/WASM/style/document relationships.
	pub entrypoints: BTreeMap<String, PagesEntrypoint>,
	/// Ordered classifier/processor identities and options.
	pub pipeline: Vec<ProcessorIdentity>,
}

/// Actual legacy reader contract; no generation guarantees are inferred (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyManifestShape {
	/// Version 1 paths.
	VersionOne,
	/// Unversioned paths.
	Paths,
	/// A legacy `files` object.
	Files,
	/// Direct logical-name/path entries.
	Flat,
}

/// Explicit legacy mapping adapter (P0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAssetManifest {
	/// Original logical-to-published mapping.
	pub paths: BTreeMap<String, String>,
	/// Source shape decoded by the shared reader.
	pub shape: LegacyManifestShape,
}

/// Supported manifest versions with their actual capabilities (P0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedAssetManifest {
	/// Complete generation metadata, requiring filesystem validation before serving.
	V2(AssetManifestV2),
	/// Legacy aliases without atomic-publication or generation guarantees.
	Legacy(LegacyAssetManifest),
}

impl DecodedAssetManifest {
	/// Borrow the logical-to-published path map across supported reader formats.
	pub fn paths(&self) -> &BTreeMap<String, String> {
		match self {
			Self::V2(value) => &value.paths,
			Self::Legacy(value) => &value.paths,
		}
	}
}
