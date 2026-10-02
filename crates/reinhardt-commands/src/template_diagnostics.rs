//! Normalization of Cargo and compiler JSON diagnostics for HMR clients.

use std::path::Path;

use reinhardt_pages::hmr::{
	BuildDiagnostic, DiagnosticLevel, DiagnosticSpan, DiagnosticTarget, PatchGeneration,
};
use serde_json::Value;

const MAX_DIAGNOSTICS: usize = 128;
const MAX_RENDERED_BYTES: usize = 16 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024;

/// Converts Cargo JSON compiler messages into bounded, project-relative HMR diagnostics.
pub fn normalize_build_diagnostics(
	project_root: &Path,
	generation: PatchGeneration,
	cargo_json: &str,
) -> Vec<BuildDiagnostic> {
	let mut diagnostics = Vec::new();
	let mut total_bytes = 0usize;
	for line in cargo_json.lines() {
		if diagnostics.len() >= MAX_DIAGNOSTICS || total_bytes >= MAX_TOTAL_BYTES {
			break;
		}
		let Ok(value) = serde_json::from_str::<Value>(line) else {
			continue;
		};
		if value.get("reason").and_then(Value::as_str) != Some("compiler-message") {
			continue;
		}
		let Some(message) = value.get("message") else {
			continue;
		};
		let text = strip_ansi(
			message
				.get("message")
				.and_then(Value::as_str)
				.unwrap_or_default(),
		);
		if text.is_empty() {
			continue;
		}
		let rendered = truncate_bytes(
			&strip_ansi(
				message
					.get("rendered")
					.and_then(Value::as_str)
					.unwrap_or(&text),
			),
			MAX_RENDERED_BYTES,
		);
		let diagnostic = BuildDiagnostic {
			generation,
			target: classify_target(&value, &rendered),
			level: parse_level(message.get("level").and_then(Value::as_str)),
			message: truncate_bytes(&text, MAX_RENDERED_BYTES),
			code: message
				.get("code")
				.and_then(|code| code.get("code"))
				.and_then(Value::as_str)
				.map(str::to_owned),
			rendered,
			relative_spans: normalize_spans(project_root, message.get("spans")),
		};
		total_bytes = total_bytes.saturating_add(diagnostic.rendered.len());
		diagnostics.push(diagnostic);
	}
	diagnostics
}

fn parse_level(level: Option<&str>) -> DiagnosticLevel {
	match level {
		Some("warning") => DiagnosticLevel::Warning,
		Some("note") => DiagnosticLevel::Note,
		Some("help") => DiagnosticLevel::Help,
		_ => DiagnosticLevel::Error,
	}
}

fn classify_target(value: &Value, rendered: &str) -> DiagnosticTarget {
	let target_name = value
		.get("target")
		.and_then(|target| target.get("name"))
		.and_then(Value::as_str)
		.unwrap_or_default()
		.to_ascii_lowercase();
	let rendered_lower = rendered.to_ascii_lowercase();
	if rendered_lower.contains("wasm-bindgen") || rendered_lower.contains("wasm_bindgen") {
		DiagnosticTarget::WasmBindgen
	} else if target_name.contains("wasm") || rendered_lower.contains("wasm32") {
		DiagnosticTarget::WasmRustc
	} else if target_name.contains("template") || rendered_lower.contains("page!") {
		DiagnosticTarget::Template
	} else if value
		.get("package_id")
		.and_then(Value::as_str)
		.is_some_and(|package| package.contains("reinhardt"))
	{
		DiagnosticTarget::ServerRustc
	} else {
		DiagnosticTarget::Other
	}
}

fn normalize_spans(project_root: &Path, spans: Option<&Value>) -> Vec<DiagnosticSpan> {
	spans
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|span| {
			let file_name = span.get("file_name").and_then(Value::as_str)?;
			let relative = relative_file_name(project_root, file_name)?;
			Some(DiagnosticSpan {
				file_name: relative,
				line_start: span.get("line_start").and_then(Value::as_u64).unwrap_or(1) as u32,
				line_end: span.get("line_end").and_then(Value::as_u64).unwrap_or(1) as u32,
				column_start: span
					.get("column_start")
					.and_then(Value::as_u64)
					.unwrap_or(1) as u32,
				column_end: span.get("column_end").and_then(Value::as_u64).unwrap_or(1) as u32,
				is_primary: span
					.get("is_primary")
					.and_then(Value::as_bool)
					.unwrap_or(false),
				label: span.get("label").and_then(Value::as_str).map(strip_ansi),
			})
		})
		.collect()
}

fn relative_file_name(project_root: &Path, file_name: &str) -> Option<String> {
	let path = Path::new(file_name);
	if path
		.components()
		.any(|component| matches!(component, std::path::Component::ParentDir))
	{
		return None;
	}
	let relative = if path.is_absolute() {
		path.strip_prefix(project_root).ok()?
	} else {
		path
	};
	Some(
		relative
			.components()
			.filter_map(|component| match component {
				std::path::Component::Normal(value) => Some(value.to_string_lossy()),
				_ => None,
			})
			.collect::<Vec<_>>()
			.join("/"),
	)
}

fn strip_ansi(value: &str) -> String {
	let mut clean = String::with_capacity(value.len());
	let mut chars = value.chars();
	while let Some(ch) = chars.next() {
		if ch == '\u{1b}' {
			if chars.next() == Some('[') {
				for control in chars.by_ref() {
					if control.is_ascii_alphabetic() {
						break;
					}
				}
			}
		} else {
			clean.push(ch);
		}
	}
	clean
}

fn truncate_bytes(value: &str, max_bytes: usize) -> String {
	if value.len() <= max_bytes {
		return value.to_owned();
	}
	let mut end = max_bytes;
	while !value.is_char_boundary(end) {
		end -= 1;
	}
	format!("{}…", &value[..end])
}
