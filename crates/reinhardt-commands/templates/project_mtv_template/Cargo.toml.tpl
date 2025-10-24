[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt = { version = "0.1.0", features = ["standard", "server", "templates"] }
tokio = { version = "1.44", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
console = "0.15"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
