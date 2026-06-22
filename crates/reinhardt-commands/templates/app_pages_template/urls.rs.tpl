//! URL configuration for the {{ app_name }} application.
//!
//! This module is intentionally target-neutral. Native builds use it to
//! aggregate app-local server and client routers, while WASM builds use the
//! same client route table and reverse helpers.

use reinhardt::{ClientRouter, ServerRouter};

pub mod client_router;

#[cfg(server)]
pub mod server_router;

pub fn server_url_patterns() -> ServerRouter {
    #[cfg(server)]
    {
        server_router::server_url_patterns()
    }
    #[cfg(not(server))]
    {
        ServerRouter::new()
    }
}

pub fn client_url_patterns() -> ClientRouter {
    client_router::client_url_patterns()
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_router::reverse(name, params)
}
