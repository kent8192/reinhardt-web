//! Native management CLI for examples-todo.

#[cfg(not(target_arch = "wasm32"))]
mod native {
	use examples_todo as _;
	use reinhardt::commands::execute_from_command_line;
	use std::process;

	#[tokio::main]
	pub(super) async fn main() {
		// SAFETY: Set once at process startup before tasks are spawned.
		unsafe {
			std::env::set_var("REINHARDT_SETTINGS_MODULE", "examples_todo.config.settings");
		}

		if let Err(error) = execute_from_command_line().await {
			eprintln!("Error: {error}");
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
