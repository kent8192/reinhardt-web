//! Build script for database-integration example
//!
//! This build script checks if reinhardt is available from crates.io
//! and whether the version matches the requirement (^0.1).
//!
//! If either check fails, appropriate cfg flags are set to conditionally
//! exclude code compilation.

fn main() {
    // Check if reinhardt is available from crates.io
    if !example_common::build_check::check_reinhardt_availability_at_build_time() {
        println!("cargo:rustc-cfg=reinhardt_unavailable");
        println!("cargo:warning=database-integration example requires reinhardt from crates.io");
        println!("cargo:warning=Example code will be stubbed out");
        return;
    }

    // Check version requirement: ^0.1 (0.1.x)
    if !example_common::build_check::check_version_requirement_at_build_time("^0.1") {
        println!("cargo:rustc-cfg=reinhardt_version_mismatch");
        println!("cargo:warning=database-integration example requires reinhardt ^0.1");
        println!("cargo:warning=Example code will be stubbed out");
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
}
