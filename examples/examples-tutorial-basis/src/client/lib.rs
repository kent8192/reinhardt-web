//! WASM SPA entry point.
//!
//! [`ClientLauncher::register_routes_from_inventory`] consumes the
//! `#[routes]`-registered router at launch time and installs the SPA
//! route table on `#root`.

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

#[cfg_attr(not(feature = "msw"), wasm_bindgen(start))]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.register_routes_from_inventory()
		.launch()
}
