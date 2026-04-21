//! WASM client for {{ project_name }}

mod bootstrap;
mod router;
mod state;

pub use router::with_router;
pub use state::*;
