//! {{ app_name }} application {% if is_workspace == "true" %}crate{% else %}module{% endif %}
//!
//! A Reinhardt Pages app whose server-side and client-side code both live
//! under this directory:
//!
//! - `server` — native-only implementation details
//! - `client` — WASM-only UI modules and client services
//! - `serializers` / `server_fn` / `services` / `urls` — cross-target
//!   module surfaces that gate client/server internals explicitly

#[cfg(server)]
use reinhardt::app_config;

#[cfg(client)]
pub mod client;

#[cfg(server)]
pub mod server;

pub mod serializers;
pub mod server_fn;
pub mod services;
pub mod urls;

#[cfg(server)]
#[app_config(name = "{{ app_name }}", label = "{{ app_name }}")]
pub struct {{ camel_case_app_name }}Config;
