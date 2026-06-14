//! Reinhardt Project Management CLI for examples-tutorial-rest

use examples_tutorial_rest::config;
use reinhardt::commands::execute_from_command_line_with_settings;
use reinhardt::core::tokio;
use std::process;

#[tokio::main]
async fn main() {
	// Set settings module environment variable
	// SAFETY: This is safe because we're setting it before any other code runs
	unsafe {
		std::env::set_var(
			"REINHARDT_SETTINGS_MODULE",
			"examples_tutorial_rest.config.settings",
		);
	}

	// Ensure config module is loaded (triggers #[routes] macro)
	let _ = &config::urls::routes;

	// Execute command from command line, handing the project's composed
	// settings to the runtime so database-requiring commands resolve the
	// connection from settings/*.toml (`[core.databases.default]`).
	if let Err(e) = execute_from_command_line_with_settings(config::settings::get_settings()).await
	{
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}
