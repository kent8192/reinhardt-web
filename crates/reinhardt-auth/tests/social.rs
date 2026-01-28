//! Integration tests for social authentication module

mod claims_test;
mod config_test;
mod error_test;
mod jwks_cache_test;
mod oidc_discovery_test;
mod pkce_flow_test;
mod state_management_test;
mod token_test;

pub mod e2e;
pub mod flows;
pub mod oidc;
pub mod providers;
