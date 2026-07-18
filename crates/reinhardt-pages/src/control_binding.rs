//! Stable support types for controlled `page!` form elements.
//!
//! The `bind:` directive accepts [`Signal`](crate::reactive::Signal) values
//! directly for text, checkbox, radio, and select controls. Numeric controls
//! can additionally report rejected input through [`NumberParseError`].
//!
//! # Target parity
//!
//! This is a P2 API: the same support types and binding contract are available
//! for browser DOM controls, server rendering, and native component tests.

pub use reinhardt_core::types::page::{
	ControlBindingError, NumberParseError, NumberParseErrorKind, NumberValue,
};
