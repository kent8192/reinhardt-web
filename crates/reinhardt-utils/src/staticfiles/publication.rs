//! Unified static asset manifests and packaging (native-only, P0).
//!
//! Legacy manifests remain readable through explicit adapters. Only a complete
//! version 2 generation can supply the production Pages publication contract.

mod classify;
#[cfg(feature = "asset-publication")]
mod inventory;
mod manifest;
mod model;
#[cfg(feature = "asset-publication")]
mod pipeline;
#[cfg(feature = "asset-publication")]
mod representations;
#[cfg(feature = "asset-publication")]
mod rewrite;

pub use classify::{AssetClassifier, validate_assignments};
pub use manifest::{decode_manifest, discover_manifest, encode_manifest};
pub use model::{
	AssetBuildError, AssetCategory, AssetEncoding, AssetManifestV2, AssetMode, AssetProducer,
	AssetRecord, AssetRole, DecodedAssetManifest, LegacyAssetManifest, LegacyManifestShape,
	PagesEntrypoint, ProcessorIdentity,
};
pub use reinhardt_core::types::static_assets::{AssetUrlError, AssetUrlSnapshot};

#[cfg(feature = "asset-publication")]
pub use pipeline::{
	AnalyzedAsset, AssetInput, AssetPipeline, AssetProcessor, AssetReference, ContentSource,
	PreparedAsset, PreparedGeneration, RewriteOutput,
};
#[cfg(feature = "asset-publication")]
pub use rewrite::{relative_asset_url, resolve_asset_reference};
