//! Core DSL types, AST definitions, and reactive traits.
//!
//! This module contains:
//! - Untyped AST nodes (`PageMacro`, `FormMacro`, `HeadMacro`)
//! - Typed AST nodes (`TypedPageMacro`, `TypedFormMacro`, etc.)
//! - Reactive primitive traits (`Signal`, `Effect`, `Memo`)
//! - Common types and utilities

pub mod reactive;
pub mod types;

pub use reactive::*;
pub use types::*;
