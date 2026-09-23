//! Relocate URL-bearing members of Web App Manifests.

use super::{reference, resolve_asset_reference};
use crate::staticfiles::publication::{AssetBuildError, AssetReference};
use serde_json::Value;

pub(super) fn analyze(logical: &str, source: &str) -> Result<Vec<AssetReference>, AssetBuildError> {
	let value: Value = serde_json::from_str(source).map_err(|error| {
		AssetBuildError::input(logical, format!("Web App Manifest JSON: {error}"))
	})?;
	validate_url_members(logical, &value)?;
	let mut references = Vec::new();
	for source in asset_sources(&value) {
		if let Some(reference) = reference(logical, source, super::Site::Raw { start: 0, end: 0 })?
		{
			references.push(reference);
		}
	}
	Ok(references)
}

pub(super) fn rewrite(
	logical: &str,
	_destination: &str,
	source: &str,
	references: &[AssetReference],
	url: &impl Fn(&AssetReference) -> Result<String, AssetBuildError>,
) -> Result<String, AssetBuildError> {
	let mut value: Value = serde_json::from_str(source).map_err(|error| {
		AssetBuildError::input(logical, format!("Web App Manifest JSON: {error}"))
	})?;
	validate_url_members(logical, &value)?;
	let mut references = references.iter();
	visit_asset_sources_mut(&mut value, &mut |value| {
		if resolve_asset_reference(logical, value)?.is_some() {
			let reference = references.next().ok_or_else(|| {
				AssetBuildError::input(
					logical,
					"Web App Manifest references changed during rewrite",
				)
			})?;
			*value = url(reference)?;
		}
		Ok(())
	})?;
	if references.next().is_some() {
		return Err(AssetBuildError::input(
			logical,
			"Web App Manifest has undeclared asset references",
		));
	}
	serde_json::to_string(&value)
		.map_err(|error| AssetBuildError::input(logical, format!("Web App Manifest JSON: {error}")))
}

fn asset_sources(value: &Value) -> Vec<&str> {
	let mut sources = Vec::new();
	append_sources(value.get("icons"), &mut sources);
	append_sources(value.get("screenshots"), &mut sources);
	if let Some(handlers) = value.get("file_handlers").and_then(Value::as_array) {
		for handler in handlers {
			append_sources(handler.get("icons"), &mut sources);
		}
	}
	if let Some(shortcuts) = value.get("shortcuts").and_then(Value::as_array) {
		for shortcut in shortcuts {
			append_sources(shortcut.get("icons"), &mut sources);
		}
	}
	sources
}

fn append_sources<'a>(value: Option<&'a Value>, output: &mut Vec<&'a str>) {
	if let Some(items) = value.and_then(Value::as_array) {
		output.extend(
			items
				.iter()
				.filter_map(|item| item.get("src").and_then(Value::as_str)),
		);
	}
}

fn visit_asset_sources_mut(
	value: &mut Value,
	visitor: &mut impl FnMut(&mut String) -> Result<(), AssetBuildError>,
) -> Result<(), AssetBuildError> {
	for key in ["icons", "screenshots"] {
		if let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) {
			for item in items {
				visit_src_mut(item, visitor)?;
			}
		}
	}
	if let Some(handlers) = value.get_mut("file_handlers").and_then(Value::as_array_mut) {
		for handler in handlers {
			if let Some(icons) = handler.get_mut("icons").and_then(Value::as_array_mut) {
				for icon in icons {
					visit_src_mut(icon, visitor)?;
				}
			}
		}
	}
	if let Some(shortcuts) = value.get_mut("shortcuts").and_then(Value::as_array_mut) {
		for shortcut in shortcuts {
			if let Some(icons) = shortcut.get_mut("icons").and_then(Value::as_array_mut) {
				for icon in icons {
					visit_src_mut(icon, visitor)?;
				}
			}
		}
	}
	Ok(())
}

fn visit_src_mut(
	item: &mut Value,
	visitor: &mut impl FnMut(&mut String) -> Result<(), AssetBuildError>,
) -> Result<(), AssetBuildError> {
	if let Some(Value::String(source)) = item.get_mut("src") {
		visitor(source)?;
	}
	Ok(())
}

fn validate_url_members(logical: &str, value: &Value) -> Result<(), AssetBuildError> {
	let mut candidates = Vec::new();
	for key in ["start_url", "scope"] {
		if let Some(source) = value.get(key).and_then(Value::as_str) {
			candidates.push((format!("/{key}"), source));
		}
	}
	if let Some(shortcuts) = value.get("shortcuts").and_then(Value::as_array) {
		for (index, shortcut) in shortcuts.iter().enumerate() {
			if let Some(source) = shortcut.get("url").and_then(Value::as_str) {
				candidates.push((format!("/shortcuts/{index}/url"), source));
			}
		}
	}
	if let Some(source) = value
		.pointer("/share_target/action")
		.and_then(Value::as_str)
	{
		candidates.push(("/share_target/action".into(), source));
	}
	if let Some(handlers) = value.get("protocol_handlers").and_then(Value::as_array) {
		for (index, handler) in handlers.iter().enumerate() {
			for key in ["url", "url_template"] {
				if let Some(source) = handler.get(key).and_then(Value::as_str) {
					candidates.push((format!("/protocol_handlers/{index}/{key}"), source));
				}
			}
		}
	}
	if let Some(handlers) = value.get("file_handlers").and_then(Value::as_array) {
		for (index, handler) in handlers.iter().enumerate() {
			if let Some(source) = handler.get("action").and_then(Value::as_str) {
				candidates.push((format!("/file_handlers/{index}/action"), source));
			}
		}
	}
	if let Some(applications) = value.get("related_applications").and_then(Value::as_array) {
		for (index, application) in applications.iter().enumerate() {
			if let Some(source) = application.get("url").and_then(Value::as_str) {
				candidates.push((format!("/related_applications/{index}/url"), source));
			}
		}
	}
	for (field, source) in candidates {
		if !source.starts_with('/') && !super::external(source) {
			return Err(AssetBuildError::input(
				logical,
				format!(
					"relative Web App Manifest URL {field} cannot be preserved after relocation; use a root-relative or external URL"
				),
			));
		}
	}
	Ok(())
}
