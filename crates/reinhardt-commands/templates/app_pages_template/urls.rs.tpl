//! URL configuration for the {{ app_name }} application.
//!
//! This module is intentionally target-neutral. Native builds use it to
//! aggregate server and client route metadata, while WASM builds use the same
//! client route table and reverse helpers.

use reinhardt::{ClientRouter, ServerRouter};

use super::pages;

pub fn server_url_patterns() -> ServerRouter {
    #[cfg(server)]
    {
        super::server::urls::server_url_patterns()
    }
    #[cfg(not(server))]
    {
        ServerRouter::new()
    }
}

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new().route("placeholder", "/{{ app_name }}/", pages::placeholder_page)
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_url_patterns()
        .reverse(name, params)
        .unwrap_or_else(|error| panic!("failed to reverse {{ app_name }} client route `{name}`: {error}"))
}
