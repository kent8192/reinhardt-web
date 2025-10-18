[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt = "{{ reinhardt_version }}"
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
