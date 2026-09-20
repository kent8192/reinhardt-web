//! Canonical decoded parents, deterministic encoded variants, and source maps.

pub(super) mod maps;

use super::classify::AssetClassifier;
use super::inventory::digest_reader;
use super::model::*;
use super::pipeline::{PreparedAsset, inferred_mime};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use tempfile::TempDir;

#[derive(Clone)]
pub(super) struct Representation {
	pub parent: String,
	pub encoding: AssetEncoding,
}

pub(super) fn identity() -> ProcessorIdentity {
	ProcessorIdentity {
		id: "reinhardt.representations".into(),
		version: "1".into(),
		options: serde_json::json!({"gzip_level":6,"gzip_mtime":0,"brotli_quality":6,"brotli_window":22,"source_map_contract":1}),
	}
}

fn representation(name: &str, classifier: &AssetClassifier) -> Option<Representation> {
	let lower = name.to_ascii_lowercase();
	if lower.ends_with(".svgz") {
		return Some(Representation {
			parent: format!("{}.svg", &name[..name.len() - 5]),
			encoding: AssetEncoding::Gzip,
		});
	}
	let (suffix, encoding) = if lower.ends_with(".gz") {
		(3, AssetEncoding::Gzip)
	} else if lower.ends_with(".br") {
		(3, AssetEncoding::Brotli)
	} else {
		return None;
	};
	classifier
		.recognizes_representation_parent(&lower[..lower.len() - suffix])
		.then(|| Representation {
			parent: name[..name.len() - suffix].into(),
			encoding,
		})
}

pub(super) fn prepare(
	assets: &mut BTreeMap<String, PreparedAsset>,
	variants: &mut BTreeMap<String, Representation>,
	spool: &TempDir,
	classifier: &AssetClassifier,
) -> Result<(), AssetBuildError> {
	let candidates: Vec<_> = assets
		.keys()
		.filter(|key| !variants.contains_key(*key))
		.filter_map(|key| representation(key, classifier).map(|value| (key.clone(), value)))
		.collect();
	for (name, variant) in candidates {
		if variants.values().any(|existing| {
			existing.parent == variant.parent && existing.encoding == variant.encoding
		}) {
			return Err(AssetBuildError::input(
				&name,
				format!(
					"duplicate {:?} representation of {}",
					variant.encoding, variant.parent
				),
			));
		}
		let asset = &assets[&name];
		let mut decoded = tempfile::NamedTempFile::new_in(spool.path())
			.map_err(|e| AssetBuildError::io(spool.path(), e))?;
		let mut reader: Box<dyn Read> = match variant.encoding {
			AssetEncoding::Gzip => Box::new(flate2::read::MultiGzDecoder::new(asset.open()?)),
			AssetEncoding::Brotli => Box::new(brotli::Decompressor::new(asset.open()?, 65536)),
		};
		std::io::copy(&mut reader, &mut decoded).map_err(|e| {
			AssetBuildError::input(&name, format!("invalid encoded representation: {e}"))
		})?;
		if let Some(parent) = assets.get(&variant.parent) {
			if parent.producer != asset.producer
				|| digest_reader(parent.open()?, &parent.path)?
					!= digest_reader(
						decoded
							.reopen()
							.map_err(|e| AssetBuildError::io(decoded.path(), e))?,
						decoded.path(),
					)? {
				return Err(AssetBuildError::input(
					&name,
					format!(
						"encoded bytes or producer disagree with canonical parent {}",
						variant.parent
					),
				));
			}
		} else {
			let role = if variant.parent.to_ascii_lowercase().ends_with(".map") {
				AssetRole::SourceMap
			} else {
				asset.role
			};
			let producer = asset.producer;
			let mime = inferred_mime(&variant.parent, role);
			// The enclosing spool guard owns the persisted decoded file from here.
			let path = decoded
				.into_temp_path()
				.keep()
				.map_err(|e| AssetBuildError::io(spool.path(), e.error))?;
			assets.insert(
				variant.parent.clone(),
				PreparedAsset {
					logical: variant.parent.clone(),
					path,
					producer,
					role,
					mime,
				},
			);
		}
		let parent = &assets[&variant.parent];
		let (role, mime) = (parent.role, parent.mime.clone());
		let encoded = assets.get_mut(&name).expect("captured representation");
		encoded.role = role;
		encoded.mime = mime;
		variants.insert(name, variant);
	}
	Ok(())
}

pub(super) fn regenerate(
	assets: &BTreeMap<String, PreparedAsset>,
	variants: &BTreeMap<String, Representation>,
) -> Result<(), AssetBuildError> {
	for (name, variant) in variants {
		let target = &assets[name].path;
		let output = File::create(target).map_err(|e| AssetBuildError::io(target, e))?;
		let mut source = assets[&variant.parent].open()?;
		match variant.encoding {
			AssetEncoding::Gzip => {
				let mut writer = flate2::GzBuilder::new()
					.mtime(0)
					.write(output, flate2::Compression::new(6));
				std::io::copy(&mut source, &mut writer)
					.map_err(|e| AssetBuildError::io(target, e))?;
				writer
					.finish()
					.map_err(|e| AssetBuildError::io(target, e))?;
			}
			AssetEncoding::Brotli => {
				let parameters = brotli::enc::BrotliEncoderParams {
					quality: 6,
					lgwin: 22,
					..Default::default()
				};
				brotli::BrotliCompress(&mut source, &mut { output }, &parameters)
					.map_err(|e| AssetBuildError::io(target, e))?;
			}
		}
	}
	Ok(())
}
