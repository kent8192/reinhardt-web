//! Parsing logic for DSL macros.
//!
//! This module provides parsers for:
//! - `page!` macro → `PageMacro`
//! - `form!` macro → `FormMacro`
//! - `head!` macro → `HeadMacro`
//! - `style!` macro → `StyleMacro`

mod client_form_view;
mod form;
mod head;
mod page;
mod style;

pub use client_form_view::*;
pub use form::*;
pub use head::*;
pub use page::*;
pub use style::*;
