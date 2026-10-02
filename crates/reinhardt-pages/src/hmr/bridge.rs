//! Development bridge between the WASM registry and the injected HMR client.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
	patch_transaction::PatchTransaction,
	protocol::{ClientHello, TemplatePatchBatch},
	template_registry::TemplateRegistry,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::Document;

type PatchCallback = Closure<dyn FnMut(JsValue) -> js_sys::Promise>;
type DiagnosticCallback = Closure<dyn FnMut(JsValue, JsValue)>;

thread_local! {
	static ACTIVE_BRIDGE: RefCell<Option<HmrBridge>> = const { RefCell::new(None) };
	static HELLO_CALLBACK: RefCell<Option<Closure<dyn FnMut() -> JsValue>>> = const { RefCell::new(None) };
	static PATCH_CALLBACK: RefCell<Option<PatchCallback>> = const { RefCell::new(None) };
	static DIAGNOSTIC_CALLBACK: RefCell<Option<DiagnosticCallback>> = const { RefCell::new(None) };
	static DIAGNOSTIC_OVERLAY: RefCell<Option<Rc<RefCell<super::overlay::ShadowDiagnosticOverlay>>>> = const { RefCell::new(None) };
}

/// Owns the development registry exposed to JavaScript callbacks.
#[cfg(wasm)]
#[derive(Clone, Default)]
pub struct HmrBridge {
	registry: TemplateRegistry,
}

#[cfg(wasm)]
impl HmrBridge {
	/// Creates a bridge with an empty registry.
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the registry used by page mounting.
	pub fn registry(&self) -> &TemplateRegistry {
		&self.registry
	}

	/// Returns the current client handshake payload.
	pub fn client_hello(&self) -> ClientHello {
		self.registry.client_hello()
	}

	/// Installs the bridge callbacks consumed by the injected HMR client.
	pub fn install(&self, document: &Document) -> Result<(), JsValue> {
		let bridge = self.clone();
		let hello_callback = Closure::wrap(Box::new(move || {
			serde_wasm_bindgen::to_value(&bridge.client_hello())
				.unwrap_or_else(|_| JsValue::from_str("{}"))
		}) as Box<dyn FnMut() -> JsValue>);
		js_sys::Reflect::set(
			&js_sys::global(),
			&JsValue::from_str("__REINHARDT_HMR_CLIENT_HELLO__"),
			hello_callback.as_ref(),
		)?;
		HELLO_CALLBACK.with(|slot| *slot.borrow_mut() = Some(hello_callback));

		let patch_registry = self.registry.clone();
		let patch_callback = Closure::wrap(Box::new(move |value: JsValue| {
			let result = serde_wasm_bindgen::from_value::<TemplatePatchBatch>(value)
				.map_err(|error| format!("decode template patch: {error}"))
				.and_then(|batch| {
					let mut mounted_patches = Vec::new();
					for patch in batch.patches {
						if patch_registry.has_descriptor(&patch.key) {
							mounted_patches.push(patch);
						} else {
							patch_registry.defer_patch(patch, batch.generation);
						}
					}
					if mounted_patches.is_empty() {
						return Ok(());
					}
					let mounted_batch = TemplatePatchBatch {
						build_id: batch.build_id,
						manifest_digest: batch.manifest_digest,
						generation: batch.generation,
						patches: mounted_patches,
					};
					let plan = PatchTransaction::plan(&mounted_batch, &patch_registry)
						.map_err(|error| error.to_string())?;
					PatchTransaction::commit(plan).map_err(|error| error.to_string())
				});
			match result {
				Ok(()) => js_sys::Promise::resolve(&JsValue::UNDEFINED),
				Err(error) => js_sys::Promise::reject(&JsValue::from_str(&error)),
			}
		}) as Box<dyn FnMut(JsValue) -> js_sys::Promise>);
		js_sys::Reflect::set(
			&js_sys::global(),
			&JsValue::from_str("__REINHARDT_HMR_PATCH_APPLIER__"),
			patch_callback.as_ref(),
		)?;
		if let Ok(value) = js_sys::Reflect::get(
			&js_sys::global(),
			&JsValue::from_str("__REINHARDT_HMR_READY__"),
		) && let Ok(register) = value.dyn_into::<js_sys::Function>()
		{
			register.call1(&js_sys::global(), patch_callback.as_ref())?;
		}
		PATCH_CALLBACK.with(|slot| *slot.borrow_mut() = Some(patch_callback));

		let overlay = Rc::new(RefCell::new(super::overlay::ShadowDiagnosticOverlay::new(
			document,
		)?));
		let overlay_for_callback = Rc::clone(&overlay);
		let diagnostic_callback =
			Closure::wrap(Box::new(move |generation: JsValue, values: JsValue| {
				let Some(generation) = generation.as_f64() else {
					return;
				};
				let generation = super::protocol::PatchGeneration(generation as u64);
				let diagnostics: Vec<super::protocol::BuildDiagnostic> =
					serde_wasm_bindgen::from_value(values).unwrap_or_default();
				let result = if diagnostics.is_empty() {
					overlay_for_callback.borrow_mut().clear(generation)
				} else {
					overlay_for_callback
						.borrow_mut()
						.apply(generation, diagnostics)
				};
				if let Err(error) = result {
					web_sys::console::error_1(&error);
				}
			}) as Box<dyn FnMut(JsValue, JsValue)>);
		if let Ok(value) = js_sys::Reflect::get(
			&js_sys::global(),
			&JsValue::from_str("__REINHARDT_HMR_DIAGNOSTICS__"),
		) && let Ok(diagnostics_register) = value.dyn_into::<js_sys::Function>()
		{
			diagnostics_register.call1(&js_sys::global(), diagnostic_callback.as_ref())?;
		}
		DIAGNOSTIC_CALLBACK.with(|slot| *slot.borrow_mut() = Some(diagnostic_callback));
		DIAGNOSTIC_OVERLAY.with(|slot| *slot.borrow_mut() = Some(overlay));
		ACTIVE_BRIDGE.with(|slot| *slot.borrow_mut() = Some(self.clone()));
		Ok(())
	}
}

/// Notifies the injected script that descriptor registration changed its
/// handshake identity.
#[cfg(wasm)]
pub(crate) fn notify_client_hello_changed() {
	if let Ok(value) = js_sys::Reflect::get(
		&js_sys::global(),
		&JsValue::from_str("__REINHARDT_HMR_CLIENT_HELLO_CHANGED__"),
	) && let Ok(callback) = value.dyn_into::<js_sys::Function>()
	{
		let _ = callback.call0(&js_sys::global());
	}
}

/// Returns the registry installed by the active client launcher, if any.
#[cfg(wasm)]
pub(crate) fn active_registry() -> Option<TemplateRegistry> {
	ACTIVE_BRIDGE.with(|slot| slot.borrow().as_ref().map(|bridge| bridge.registry.clone()))
}
