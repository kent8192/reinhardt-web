//! Unified static asset manifests and classification (native-only, P0).
//!
//! Legacy manifests remain readable through explicit adapters. Version 2 describes
//! complete generations with producer-aware asset categories and dependency edges.

mod classify;
mod manifest;
mod model;

pub use classify::{AssetClassifier, validate_assignments};
pub use manifest::{decode_manifest, discover_manifest, encode_manifest};
pub use model::{
	AssetBuildError, AssetCategory, AssetEncoding, AssetManifestV2, AssetMode, AssetProducer,
	AssetRecord, AssetRole, DecodedAssetManifest, LegacyAssetManifest, LegacyManifestShape,
	PagesEntrypoint, ProcessorIdentity,
};
pub use reinhardt_core::types::static_assets::{AssetUrlError, AssetUrlSnapshot};
