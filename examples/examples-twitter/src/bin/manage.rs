//! Reinhardt Project Management CLI for examples-twitter

#[cfg(not(target_arch = "wasm32"))]
mod native {
	use examples_twitter as _;
	use examples_twitter::config::settings::get_settings;
	use reinhardt::commands::execute_from_command_line_with_settings;
	use std::process;

	#[tokio::main]
	pub(super) async fn main() {
		// SAFETY: Called at program start before any spawned tasks.
		unsafe {
			std::env::set_var(
				"REINHARDT_SETTINGS_MODULE",
				"examples_twitter.config.settings",
			);
		}

		if let Err(e) = execute_from_command_line_with_settings(get_settings()).await {
			eprintln!("Error: {e}");
			process::exit(1);
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	native::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
