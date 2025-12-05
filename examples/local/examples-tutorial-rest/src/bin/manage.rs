//! Reinhardt Project Management CLI for examples-tutorial-rest
//!
//! This is the project-specific management command interface (equivalent to Django's manage.py).

use examples_tutorial_rest::config::urls::url_patterns;
use reinhardt::commands::execute_from_command_line;
use reinhardt::core::tokio;
use reinhardt::urls::routers::register_router_arc;
use std::process;

#[tokio::main]
async fn main() {
	// Set settings module environment variable
	// SAFETY: This is safe because we're setting it before any other code runs
	unsafe {
		std::env::set_var("REINHARDT_SETTINGS_MODULE", "examples_tutorial_rest.config.settings");
	}

	// Register router before executing commands
	register_router_arc(url_patterns());

	// Execute command from command line
	if let Err(e) = execute_from_command_line().await {
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}
