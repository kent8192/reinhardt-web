//! Multi-runtime plugin execution layer.
//!
//! This module provides the runtime abstraction layer that enables plugins
//! to run on different backends:
//!
//! - **Static**: Rust plugins compiled with the host application
//! - **WASM**: WebAssembly plugins via wasmtime
//! - **TypeScript**: TypeScript/JavaScript plugins via deno_core
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     PluginRuntime                           │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
//! │  │   Static   │  │    WASM    │  │ TypeScript │            │
//! │  │  Runtime   │  │  Runtime   │  │  Runtime   │            │
//! │  │   (Rust)   │  │ (wasmtime) │  │(deno_core) │            │
//! │  └────────────┘  └────────────┘  └────────────┘            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_dentdelion::runtime::{PluginRuntime, RuntimeType};
//!
//! fn describe_runtime(runtime: &dyn PluginRuntime) {
//!     match runtime.runtime_type() {
//!         RuntimeType::Static => println!("Native Rust execution"),
//!         RuntimeType::Wasm => println!("Sandboxed WASM execution"),
//!         RuntimeType::TypeScript => println!("V8 TypeScript execution"),
//!     }
//! }
//! ```

mod abstraction;

pub use abstraction::*;
