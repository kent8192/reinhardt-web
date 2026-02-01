//! Test that direct use of #[derive(AppConfig)] produces a compile error
//!
//! Direct derive usage is not allowed. Users must use #[app_config(...)] instead.

use reinhardt_macros::AppConfig;

// This should fail to compile with a helpful error message
#[derive(AppConfig)]
#[app_config(name = "test", label = "test")]
pub struct DirectDeriveConfig;

fn main() {}
