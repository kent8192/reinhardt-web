//! WASM client module
//!
//! This module contains the WASM client entry point and shared client components.
pub mod components;
pub mod lib;
pub mod pages;
pub mod router;
pub use lib::main;
pub use router::{init_global_router, with_router};
