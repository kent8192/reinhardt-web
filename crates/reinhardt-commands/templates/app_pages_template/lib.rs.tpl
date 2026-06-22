//! {{ app_name }} application {% if is_workspace == "true" %}crate{% else %}module{% endif %}
//!
//! A Reinhardt Pages app whose server-side and client-side code both live
//! under this directory:
//!
//! - `admin` / `models` / `serializers` / `services` / `views` — server-only
//! - `server_fn` / `urls` — bi-target (gate internally)
//! - `client` — WASM-only (per-app UI + page wrappers)

#[cfg(server)]
use reinhardt::app_config;

#[cfg(server)]
pub mod admin;
#[cfg(server)]
pub mod models;
#[cfg(server)]
pub mod serializers;
#[cfg(server)]
pub mod services;
#[cfg(server)]
pub mod views;

// Bi-target modules: both server and client portions live inside, gated internally.
pub mod server_fn;
pub mod urls;

#[cfg(client)]
pub mod client;

#[cfg(server)]
#[app_config(name = "{{ app_name }}", label = "{{ app_name }}")]
pub struct {{ camel_case_app_name }}Config;
