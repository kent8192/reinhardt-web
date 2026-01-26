//! WASM Test Module
//!
//! This module contains WASM-specific tests that run in a browser environment
//! using `wasm-bindgen-test`. These tests verify functionality that requires
//! actual DOM access, such as CSRF token retrieval from cookies, meta tags,
//! and form inputs.

#![cfg(target_arch = "wasm32")]

pub mod csrf_wasm_test;
pub mod server_fn_wasm_test;
