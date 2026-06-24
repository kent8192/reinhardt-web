//! URL configuration for the {{ app_name }} application.
//!
//! Server and client route implementations stay split so each side can use
//! target-specific modules without leaking them across cfg boundaries.

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
