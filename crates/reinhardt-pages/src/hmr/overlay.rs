//! Shadow DOM compiler-diagnostic overlay.

use wasm_bindgen::JsValue;
use web_sys::{Element, ShadowRoot, ShadowRootInit, ShadowRootMode};

use super::{diagnostics::DiagnosticStore, protocol::PatchGeneration};

/// Isolated compiler diagnostics host that never interpolates compiler text as HTML.
#[cfg(wasm)]
pub struct ShadowDiagnosticOverlay {
	host: Element,
	shadow: ShadowRoot,
	store: DiagnosticStore,
}

#[cfg(wasm)]
impl ShadowDiagnosticOverlay {
	/// Creates and attaches an accessible diagnostic host to the document body.
	pub fn new(document: &web_sys::Document) -> Result<Self, JsValue> {
		let host = document.create_element("div")?;
		host.set_id("reinhardt-hmr-diagnostics");
		host.set_attribute("role", "alert")?;
		host.set_attribute("aria-live", "polite")?;
		let shadow = host.attach_shadow(&ShadowRootInit::new(ShadowRootMode::Open))?;
		document
			.body()
			.ok_or_else(|| JsValue::from_str("document body is unavailable"))?
			.append_child(&host)?;
		Ok(Self {
			host,
			shadow,
			store: DiagnosticStore::new(),
		})
	}

	/// Applies diagnostics for a generation and rerenders the isolated panel.
	pub fn apply(
		&mut self,
		generation: PatchGeneration,
		diagnostics: Vec<super::protocol::BuildDiagnostic>,
	) -> Result<bool, JsValue> {
		if !self.store.apply(generation, diagnostics) {
			return Ok(false);
		}
		self.render()?;
		Ok(true)
	}

	/// Clears the panel after a successful build or patch.
	pub fn clear(&mut self, generation: PatchGeneration) -> Result<bool, JsValue> {
		if !self.store.clear(generation) {
			return Ok(false);
		}
		self.render()?;
		Ok(true)
	}

	/// Returns the host element for external lifecycle integration.
	pub fn host(&self) -> &Element {
		&self.host
	}

	fn render(&self) -> Result<(), JsValue> {
		while let Some(child) = self.shadow.first_child() {
			self.shadow.remove_child(&child)?;
		}
		if self.store.diagnostics().is_empty() {
			return Ok(());
		}
		let document = web_sys::window()
			.ok_or_else(|| JsValue::from_str("window is unavailable"))?
			.document()
			.ok_or_else(|| JsValue::from_str("document is unavailable"))?;
		let panel = document.create_element("section")?;
		panel.set_attribute("data-reinhardt-hmr", "diagnostics")?;
		for diagnostic in self.store.diagnostics() {
			let item = document.create_element("article")?;
			let location = diagnostic
				.relative_spans
				.first()
				.map(|span| {
					format!(
						"{}:{}:{}",
						span.file_name, span.line_start, span.column_start
					)
				})
				.unwrap_or_else(|| "unknown location".to_owned());
			let header = format!(
				"{:?} {:?} {}{}",
				diagnostic.target,
				diagnostic.level,
				location,
				diagnostic
					.code
					.as_deref()
					.map(|code| format!(" ({code})"))
					.unwrap_or_default()
			);
			let header_node = document.create_element("strong")?;
			header_node.set_text_content(Some(&header));
			item.append_child(&header_node)?;
			let message_node = document.create_element("pre")?;
			message_node.set_text_content(Some(&diagnostic.rendered));
			item.append_child(&message_node)?;
			panel.append_child(&item)?;
		}
		self.shadow.append_child(&panel)?;
		Ok(())
	}
}
