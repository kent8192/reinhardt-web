//! Test that `asset()` without `url` is rejected with a clear message.

use reinhardt_macros::app_config;

#[app_config(
	name = "bad",
	label = "bad",
	vendor_assets(
		asset(target = "vendor/x.js"),
	),
)]
pub struct BadConfig;

fn main() {}
