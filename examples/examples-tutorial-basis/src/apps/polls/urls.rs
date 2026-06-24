//! URL configuration for the polls application.

#[cfg(not(client))]
mod client_route_specs;

#[cfg(client)]
pub mod client_router;

#[cfg(server)]
pub mod server_router;

#[cfg(not(client))]
pub use client_route_specs::{client_url_patterns, reverse};
#[cfg(client)]
pub use client_router::{client_url_patterns, reverse};

#[cfg(server)]
pub use server_router::server_url_patterns;
