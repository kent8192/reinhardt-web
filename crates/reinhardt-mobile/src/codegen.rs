//! Code generation for reinhardt-mobile.
//!
//! This module provides code generation utilities for transforming
//! reinhardt-manouche IR into mobile-specific code and assets.
//!
//! # Components
//!
//! - [`MobileVisitor`] - IRVisitor implementation for mobile code generation
//! - [`AssetGenerator`] - HTML/CSS/JS asset generation
//! - [`IpcCodeGenerator`] - IPC bridge code generation
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_mobile::codegen::{MobileVisitor, AssetGenerator};
//! use reinhardt_mobile::MobileConfig;
//!
//! let config = MobileConfig::default();
//! let mut visitor = MobileVisitor::new(config.clone());
//! let assets = AssetGenerator::new(config).generate()?;
//! ```

mod assets;
mod ipc;
mod visitor;

pub use assets::{AssetGenerator, MobileAssets};
pub use ipc::{IpcCodeGenerator, IpcCommandDef, generate_ipc_bridge};
pub use visitor::MobileVisitor;
