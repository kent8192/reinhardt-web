//! Client-side (WASM) modules for {{ project_name }}.
//!
//! - `lib`        — `#[wasm_bindgen(start)]` entry point (delegates to `ClientLauncher`)
//! - `components` — reusable UI components grouped per app

pub mod lib;

pub mod components;
