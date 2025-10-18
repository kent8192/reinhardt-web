[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt-core = { path = "../../reinhardt-core" }
reinhardt-routers = { path = "../../reinhardt-routers" }
reinhardt-template = { path = "../../reinhardt-template" }
tokio = { version = "1.44", features = ["full"] }
hyper = { version = "1.5", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
askama = "0.14.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
