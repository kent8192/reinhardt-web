//! Reinhardt Project Management CLI for examples-tutorial-rest
use examples_tutorial_rest::config;
use reinhardt::commands::execute_from_command_line;
use reinhardt::core::tokio;
use std::process;
#[tokio::main]
async fn main() {
	unsafe {
		std::env::set_var(
			"REINHARDT_SETTINGS_MODULE",
			"examples_tutorial_rest.config.settings",
		);
	}
	let _ = &config::urls::routes;
	if let Err(e) = execute_from_command_line().await {
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}
