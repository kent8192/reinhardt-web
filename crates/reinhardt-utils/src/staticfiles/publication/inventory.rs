//! Canonical generation inventory, excluding its own ID and deployment location.

use super::manifest::canonical_json;
use super::model::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

#[derive(Serialize)]
struct Inventory<'a> {
	contract: &'static str,
	mode: AssetMode,
	paths: &'a BTreeMap<String, String>,
	assets: &'a BTreeMap<String, AssetRecord>,
	entrypoints: &'a BTreeMap<String, PagesEntrypoint>,
	pipeline: &'a [ProcessorIdentity],
}

pub(super) fn build_id(
	mode: AssetMode,
	paths: &BTreeMap<String, String>,
	assets: &BTreeMap<String, AssetRecord>,
	entrypoints: &BTreeMap<String, PagesEntrypoint>,
	pipeline: &[ProcessorIdentity],
) -> Result<String, AssetBuildError> {
	let bytes = canonical_json(&Inventory {
		contract: "reinhardt.static-assets/2",
		mode,
		paths,
		assets,
		entrypoints,
		pipeline,
	})?;
	Ok(hex::encode(Sha256::digest(bytes)))
}

pub(super) fn digest_reader(
	mut reader: impl Read,
	path: &Path,
) -> Result<(u64, String), AssetBuildError> {
	let mut hash = Sha256::new();
	let mut size = 0u64;
	let mut buffer = [0u8; 64 * 1024];
	loop {
		let read = reader
			.read(&mut buffer)
			.map_err(|e| AssetBuildError::io(path, e))?;
		if read == 0 {
			break;
		}
		hash.update(&buffer[..read]);
		size += read as u64;
	}
	Ok((size, hex::encode(hash.finalize())))
}
