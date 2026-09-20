//! Data-only HTML rendering: syntax parsing belongs to the packaging feature.

use super::{AssetBuildError, AssetManifestV2, AssetRole, ManifestSnapshot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Versioned rendering instructions stored inside an entry asset record (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentProgram {
	/// Instruction version, currently `1`.
	pub version: u32,
	/// Serialized HTML and typed URL/injection slots in document order.
	pub chunks: Vec<DocumentChunk>,
	/// Logical stylesheet links already present in the template, in source order.
	pub styles: Vec<String>,
}

/// HTML render instruction; these are compiled from trusted application HTML (P0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DocumentChunk {
	/// Literal serialized template text.
	Text {
		/// HTML source.
		value: String,
	},
	/// Resolve one logical URL using the request's immutable projection.
	Asset {
		/// Logical dependency name.
		logical: String,
		/// Original query/fragment suffix.
		suffix: String,
		/// Destination syntax context.
		escaping: DocumentEscaping,
	},
	/// Safe JSON projection before application module evaluation.
	Bootstrap,
	/// Ordered entry styles not already linked by the template.
	Styles,
	/// Framework Pages module initializer.
	Loader,
}

/// Escaping of a URL inside a precompiled HTML render instruction (P0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentEscaping {
	/// Double-quoted HTML attribute contents.
	HtmlAttribute,
	/// Double-quoted JavaScript string contents in a script element.
	JavaScript,
	/// Double-quoted CSS string contents in a style element.
	Css,
	/// CSS string contents nested in an HTML attribute.
	CssAttribute,
	/// A source map directive in an inline comment.
	Comment,
}

pub(super) fn validate_program(
	name: &str,
	manifest: &AssetManifestV2,
) -> Result<(), AssetBuildError> {
	let record = &manifest.assets[name];
	if record.role != AssetRole::EntryDocument || record.encoding.is_some() {
		if record.document.is_some() {
			return Err(AssetBuildError::manifest(format!(
				"noncanonical document program on {name:?}"
			)));
		}
		return Ok(());
	}
	let program = record.document.as_ref().ok_or_else(|| {
		AssetBuildError::manifest(format!(
			"entry document {name:?} lacks render instructions; rebuild with buildstatic"
		))
	})?;
	if record.mime.split(';').next() != Some("text/html") || program.version != 1 {
		return Err(AssetBuildError::manifest(format!(
			"unsupported document program on {name:?}"
		)));
	}
	let mut slots = [0; 3];
	for chunk in &program.chunks {
		match chunk {
			DocumentChunk::Bootstrap => slots[0] += 1,
			DocumentChunk::Styles => slots[1] += 1,
			DocumentChunk::Loader => {
				if slots[0] != 1 {
					return Err(AssetBuildError::manifest(
						"document loader precedes its projection",
					));
				}
				slots[2] += 1;
			}
			DocumentChunk::Asset {
				logical, suffix, ..
			} => {
				if !record.dependencies.contains(logical)
					|| (!suffix.is_empty() && !suffix.starts_with(['?', '#']))
					|| suffix.chars().any(char::is_control)
				{
					return Err(AssetBuildError::manifest(format!(
						"invalid document dependency {logical:?} in {name:?}"
					)));
				}
			}
			DocumentChunk::Text { .. } => {}
		}
	}
	if slots != [1, 1, 1] {
		return Err(AssetBuildError::manifest(format!(
			"entry document {name:?} requires exactly one bootstrap, styles and loader slot"
		)));
	}
	let mut styles = BTreeSet::new();
	for style in &program.styles {
		if !styles.insert(style)
			|| !record.dependencies.contains(style)
			|| !manifest.assets.get(style).is_some_and(|record| {
				record.mime.split(';').next() == Some("text/css") && record.encoding.is_none()
			}) {
			return Err(AssetBuildError::manifest(format!(
				"invalid template stylesheet {style:?} in {name:?}"
			)));
		}
	}
	Ok(())
}

const LOADER: &str = "<script type=\"module\" id=\"reinhardt-pages-loader\">\nconst snapshot = JSON.parse(document.getElementById('reinhardt-static-assets').textContent);\nconst entry = JSON.parse(document.getElementById('reinhardt-pages-entry').textContent);\nconst module = await import(entry.javascript);\nawait module.default({ module_or_path: entry.wasm });\n</script>";

fn default_program() -> DocumentProgram {
	DocumentProgram {
		version: 1,
		styles: Vec::new(),
		chunks: vec![
			DocumentChunk::Text {
				value: "<!DOCTYPE html><html><head><meta charset=\"utf-8\">".into(),
			},
			DocumentChunk::Bootstrap,
			DocumentChunk::Styles,
			DocumentChunk::Text {
				value: "</head><body><div id=\"app\"></div>".into(),
			},
			DocumentChunk::Loader,
			DocumentChunk::Text {
				value: "</body></html>".into(),
			},
		],
	}
}

/// Render a selected Pages entry using only its pinned, verified generation (P0).
///
/// `template` must be the selected document's captured UTF-8 bytes. When the entry
/// declares no document, pass an empty string to select the built-in shell.
/// URL projection JSON precedes every application module in packaged documents.
pub fn render_entry_document(
	snapshot: &ManifestSnapshot,
	static_url: &str,
	entrypoint: &str,
	template: &str,
) -> Result<String, AssetBuildError> {
	let manifest = snapshot.manifest();
	let entry = manifest
		.entrypoints
		.get(entrypoint)
		.ok_or_else(|| AssetBuildError::input(entrypoint, "unknown Pages entrypoint"))?;
	let fallback;
	let program = if let Some(name) = &entry.document {
		let record = &manifest.assets[name];
		if hex::encode(Sha256::digest(template.as_bytes())) != record.sha256 {
			return Err(AssetBuildError::input(
				name,
				"entry template does not belong to the selected generation",
			));
		}
		record.document.as_ref().ok_or_else(|| {
			AssetBuildError::input(name, "missing document program; rebuild with buildstatic")
		})?
	} else {
		if !template.is_empty() {
			return Err(AssetBuildError::input(
				entrypoint,
				"entry has no declared document; pass an empty template",
			));
		}
		fallback = default_program();
		&fallback
	};
	let projection = snapshot.url_snapshot(static_url)?;
	let entry_json = serde_json::json!({ "javascript": projection.resolve(&entry.javascript)?, "wasm": projection.resolve(&entry.wasm)? });
	let mut output = String::new();
	for chunk in &program.chunks {
		match chunk {
			DocumentChunk::Text { value } => output.push_str(value),
			DocumentChunk::Asset {
				logical,
				suffix,
				escaping,
			} => {
				output.push_str(&escape_url(
					&format!("{}{suffix}", projection.resolve(logical)?),
					*escaping,
				));
			}
			DocumentChunk::Bootstrap => {
				output
					.push_str("<script type=\"application/json\" id=\"reinhardt-static-assets\">");
				output.push_str(&safe_json(&projection.to_json()?));
				output.push_str(
					"</script><script type=\"application/json\" id=\"reinhardt-pages-entry\">",
				);
				output.push_str(&safe_json(&entry_json.to_string()));
				output.push_str("</script>");
			}
			DocumentChunk::Styles => {
				for style in &entry.styles {
					if !program.styles.contains(style) {
						output.push_str("<link rel=\"stylesheet\" href=\"");
						output.push_str(&escape_attribute(&projection.resolve(style)?));
						output.push_str("\">");
					}
				}
			}
			DocumentChunk::Loader => output.push_str(LOADER),
		}
	}
	Ok(output)
}

fn safe_json(json: &str) -> String {
	json.replace('&', "\\u0026")
		.replace('<', "\\u003c")
		.replace('>', "\\u003e")
		.replace('\u{2028}', "\\u2028")
		.replace('\u{2029}', "\\u2029")
}

pub(super) fn escape_attribute(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
}

fn escape_url(value: &str, context: DocumentEscaping) -> String {
	match context {
		DocumentEscaping::HtmlAttribute => escape_attribute(value),
		DocumentEscaping::JavaScript => {
			let quoted = safe_json(&serde_json::to_string(value).expect("string serialization"));
			quoted[1..quoted.len() - 1].into()
		}
		DocumentEscaping::Css | DocumentEscaping::CssAttribute => {
			let mut escaped = String::new();
			for ch in value.chars() {
				match ch {
					'"' | '\\' => {
						escaped.push('\\');
						escaped.push(ch);
					}
					ch if ch.is_control() || matches!(ch, '<' | '>') => {
						use std::fmt::Write;
						write!(escaped, "\\{:x} ", ch as u32).expect("String write");
					}
					_ => escaped.push(ch),
				}
			}
			if context == DocumentEscaping::CssAttribute {
				escape_attribute(&escaped)
			} else {
				escaped
			}
		}
		DocumentEscaping::Comment => value
			.replace('*', "%2A")
			.replace('<', "%3C")
			.replace('>', "%3E")
			.replace(['\r', '\n'], ""),
	}
}
