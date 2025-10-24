[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt = { version = "{{ reinhardt_version }}", features = ["standard", "server"] }
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
console = "0.15"
