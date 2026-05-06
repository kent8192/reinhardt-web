[package]
name = "{{NAME}}"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "manage"
path = "src/main.rs"

[target.'cfg(target_arch = "wasm32")'.dependencies]
# Pinned to the exact version of `wasm-bindgen-cli` available on the test
# host. The test harness probes the locally-installed CLI's version at
# runtime and rewrites this token before materialising the fixture, so
# this literal acts as a fallback for the grep `cargo update` advice
# rather than as the actual pin.
wasm-bindgen = "={{WASM_BINDGEN_VERSION}}"

[workspace]
