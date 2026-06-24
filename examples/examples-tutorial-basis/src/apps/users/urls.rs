//! URL configuration for the users application.

#[cfg(client)]
pub mod client_router;

#[cfg(server)]
pub mod server_router;

#[cfg(client)]
pub use client_router::{client_url_patterns, reverse};

#[cfg(server)]
pub use server_router::server_url_patterns;
