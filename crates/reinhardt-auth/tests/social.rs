//! Integration tests for social authentication module

mod pkce_flow_test;
mod state_management_test;
mod oidc_discovery_test;
mod jwks_cache_test;
mod config_test;
mod token_test;
mod claims_test;
mod error_test;

pub mod providers;
pub mod flows;
pub mod oidc;
pub mod e2e;
