//! Reinhardt Project Management CLI for examples-twitter
//!
//! This is the project-specific management command interface (equivalent to Django's manage.py).

use examples_twitter::config::settings::get_settings;
use examples_twitter::config::urls::url_patterns;
use reinhardt::commands::execute_from_command_line;
use reinhardt::core::tokio;
use reinhardt::db::DatabaseConnection;
use reinhardt::di::{InjectionContext, SingletonScope};
use reinhardt::register_di_context;
use reinhardt::register_router_arc;
use std::process;
use std::sync::Arc;

#[tokio::main]
async fn main() {
	// Set settings module environment variable
	// SAFETY: This is safe because we're setting it before any other code runs
	unsafe {
		std::env::set_var("REINHARDT_SETTINGS_MODULE", "examples-twitter.config.settings");
	}

	// Load settings
	let settings = get_settings();

	// Initialize DatabaseConnection
	let db = match initialize_database(&settings).await {
		Ok(db) => db,
		Err(e) => {
			eprintln!("Failed to initialize database: {}", e);
			process::exit(1);
		}
	};

	// Create DI context and register DatabaseConnection
	let singleton_scope = SingletonScope::new();
	singleton_scope.set(db);

	let di_context = Arc::new(InjectionContext::builder(Arc::new(singleton_scope)).build());

	// Register DI context globally
	register_di_context(di_context);

	// Get router and register
	let router = url_patterns();

	// Register router before executing commands
	register_router_arc(router);

	// Execute command from command line
	if let Err(e) = execute_from_command_line().await {
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}

/// Initialize database connection based on settings
async fn initialize_database(settings: &reinhardt::Settings) -> Result<Arc<DatabaseConnection>, Box<dyn std::error::Error + Send + Sync>> {
	// Get database URL from settings (using "default" database)
	let db_config = settings
		.databases
		.get("default")
		.ok_or_else(|| "Default database not configured in settings")?;

	// Get URL from options or build it from components
	let db_url = if let Some(url) = db_config.options.get("url") {
		url.clone()
	} else {
		// Build URL from components
		let host = db_config.host.as_deref().unwrap_or("localhost");
		let port = db_config.port.unwrap_or(5432);
		let user = db_config.user.as_deref().unwrap_or("postgres");
		let name = &db_config.name;
		format!("postgresql://{}@{}:{}/{}", user, host, port, name)
	};

	// Connect to database
	let db = DatabaseConnection::connect_postgres(&db_url).await?;

	Ok(Arc::new(db))
}
