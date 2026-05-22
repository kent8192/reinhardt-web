//! Build script for spa_navigation_with_named_routes_app.
//!
//! Sets up cfg aliases for simplified conditional compilation.
use cfg_aliases::cfg_aliases;
fn main() {
    println!("cargo::rustc-check-cfg=cfg(client)");
    println!("cargo::rustc-check-cfg=cfg(server)");
    cfg_aliases! {
        client : { target_arch = "wasm32" }, server : { not(target_arch = "wasm32") },
    }
}
