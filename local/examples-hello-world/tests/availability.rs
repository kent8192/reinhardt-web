//! Availability tests for hello-world example
//!
//! Compilation and execution control:
//! - Cargo.toml: [[test]] name = "availability" required-features = ["with-reinhardt"]
//! - build.rs: Sets 'with-reinhardt' feature when reinhardt is available
//! - When feature is disabled, this entire test file is excluded from compilation

use example_common::availability;

/// Run first: check if reinhardt can be obtained from crates.io
#[test]
fn test_reinhardt_available() {
	match availability::ensure_reinhardt_available() {
		Ok(_) => {
			println!("✅ reinhardt is available from crates.io");
		}
		Err(e) => {
			eprintln!("❌ reinhardt is NOT available from crates.io: {}", e);
			eprintln!("   All subsequent tests will be skipped.");
			eprintln!("   This is expected if reinhardt is not yet published.");

			// Don't panic before publication (displayed as warning in CI)
			eprintln!("⚠️  Skipping examples tests (reinhardt not published)");
		}
	}
}
