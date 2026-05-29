//! Native management CLI for examples-todo.

#[cfg(not(target_arch = "wasm32"))]
mod native {
	use examples_todo as _;
	use reinhardt::commands::execute_from_command_line;
	use std::process;

	#[tokio::main]
	pub(super) async fn main() {
		if let Err(error) = execute_from_command_line().await {
			eprintln!("Error: {error}");
			process::exit(1);
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	// SAFETY: Executed before the Tokio runtime is created and before any
	// additional threads are spawned, so the process environment is mutated
	// while the program is still single-threaded.
	unsafe {
		std::env::set_var("REINHARDT_SETTINGS_MODULE", "examples_todo.config.settings");
	}
	native::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
