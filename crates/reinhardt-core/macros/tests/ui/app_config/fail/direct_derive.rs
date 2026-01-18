//! Test that direct use of #[derive(AppConfig)] produces a compile error
//!
//! Direct derive usage is not allowed. Users must use #[app_config(...)] instead.

use reinhardt_macros::AppConfig;

// Allow this test crate to be referenced as `::reinhardt`
extern crate self as reinhardt;

pub mod reinhardt_apps {
	pub use ::reinhardt_apps::*;
}

pub mod macros {
	pub use reinhardt_macros::AppConfig;
}

// This should fail to compile with a helpful error message
#[derive(AppConfig)]
#[app_config(name = "test", label = "test")]
pub struct DirectDeriveConfig;

fn main() {}
