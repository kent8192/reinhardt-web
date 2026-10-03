[package]
name = "capability-migration-scope"
version = "0.0.0"
edition = "2024"

[workspace]

[[bin]]
name = "capability-migration-manage"
path = "src/bin/manage.rs"

[dependencies]
reinhardt = { path = "__REINHARDT_ROOT__", package = "reinhardt-web", default-features = false, features = ["core", "commands-contract", "db-sqlite", "auth", "sessions"] }
ctor = "0.6"
clap = { version = "4", features = ["derive"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["serde", "v4"] }
sea-query = { version = "0.32", features = ["with-uuid"] }

[profile.dev]
debug = 0
