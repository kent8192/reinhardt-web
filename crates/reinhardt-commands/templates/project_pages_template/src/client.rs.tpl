//! Client-side (WASM) modules for {{ project_name }}.
//!
//! - `lib`        — `#[wasm_bindgen(start)]` entry point (delegates to `ClientLauncher`)
//! - `router`     — client-side router definition (`init_router` + re-exported `with_router`)
//! - `pages`      — top-level page components
//! - `components` — reusable UI components grouped per app

pub mod lib;

pub mod router;

pub mod pages;

pub mod components;
