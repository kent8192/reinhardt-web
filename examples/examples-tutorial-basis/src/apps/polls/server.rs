//! Server-only implementation details for the polls application.

#[cfg(server)]
pub mod admin;
pub mod models;
#[cfg(server)]
pub mod serializers;
