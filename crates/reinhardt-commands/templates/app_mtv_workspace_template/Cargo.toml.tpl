[package]
name = "{{ app_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt-core = { workspace = true }
reinhardt-orm = { workspace = true }
reinhardt-routers = { workspace = true }
reinhardt-template = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
askama = "0.14.0"

[lib]
name = "{{ app_name }}"
path = "src/lib.rs"
