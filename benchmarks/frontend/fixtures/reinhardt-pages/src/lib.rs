#[cfg(wasm)]
mod client;

#[cfg(wasm)]
pub use client::mount_benchmark_app;
