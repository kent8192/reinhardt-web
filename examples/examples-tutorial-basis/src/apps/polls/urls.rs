//! URL configuration for the polls application.

pub mod client_router;

#[cfg(server)]
pub mod server_router;

pub use client_router::{client_url_patterns, reverse};

#[cfg(server)]
pub use server_router::server_url_patterns;
