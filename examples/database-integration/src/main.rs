//! Database Integration Example for Reinhardt
//!
//! This example demonstrates database integration using Reinhardt's ORM and migration system.

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_core::Settings;
    use std::sync::Arc;

    pub mod config;
    pub mod apps;
    mod migrations;

    /// Initialize the application with settings
    pub async fn init() -> Result<Arc<Settings>, Box<dyn std::error::Error>> {
        let settings = config::settings::get_settings();
        let settings = Arc::new(settings);

        println!("✅ Application initialized");
        println!("Debug mode: {}", settings.debug);
        println!("Database URL: {}", settings.database.as_ref().map(|d| d.url.as_str()).unwrap_or("not configured"));

        Ok(settings)
    }

    /// Run the application
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        println!("Database Integration Example");

        let _settings = init().await?;

        // Database connection would be established here
        // For now, just demonstrate the structure

        println!("✅ Application started successfully");

        Ok(())
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub use unavailable::*;

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
mod unavailable {
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("⚠️  Database Integration Example");
        eprintln!();
        eprintln!("This example requires reinhardt from crates.io (version ^0.1).");
        eprintln!();
        eprintln!("Current status:");
        #[cfg(reinhardt_unavailable)]
        eprintln!("  ❌ reinhardt is not available from crates.io");
        #[cfg(reinhardt_version_mismatch)]
        eprintln!("  ❌ reinhardt version does not match requirement ^0.1");
        eprintln!();
        eprintln!("This example will be available once reinhardt 0.1.x is published.");
        eprintln!();
        eprintln!("For development, use the integration tests in tests/ directory instead.");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}
