//! Test that basic #[app_config(...)] usage compiles successfully

use reinhardt_macros::app_config;

// Allow this test crate to be referenced as `::reinhardt`
extern crate self as reinhardt;

pub mod reinhardt_apps {
	pub use ::reinhardt_apps::*;
}

pub mod macros {
	pub use reinhardt_macros::AppConfig;
}

// Basic app config without verbose_name
#[app_config(name = "basic", label = "basic")]
pub struct BasicConfig;

// App config with verbose_name
#[app_config(name = "full", label = "full", verbose_name = "Full Application")]
pub struct FullConfig;

fn main() {
	let basic = BasicConfig::config();
	assert_eq!(basic.name, "basic");
	assert_eq!(basic.label, "basic");
	assert_eq!(basic.verbose_name, None);

	let full = FullConfig::config();
	assert_eq!(full.name, "full");
	assert_eq!(full.label, "full");
	assert_eq!(full.verbose_name, Some("Full Application".to_string()));
}
