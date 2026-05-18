//! Reinhardt Project Management CLI for examples-tutorial-basis
//!
//! This binary is intentionally native-only. The whole module body is gated
//! behind `not(target_arch = "wasm32")` so that
//! `cargo check --target wasm32-unknown-unknown` on the workspace does not
//! try to compile a tokio-based CLI for the browser target. The wasm side
//! still requires a `main` symbol for `bin` crate-types, so we keep an
//! empty stub.

#[cfg(not(target_arch = "wasm32"))]
mod native {
	use reinhardt::commands::execute_from_command_line;
	use std::process;

	#[tokio::main]
	pub(super) async fn main() {
		// SAFETY: Called at program start before any spawned tasks.
		unsafe {
			std::env::set_var(
				"REINHARDT_SETTINGS_MODULE",
				"examples_tutorial_basis.config.settings",
			);
		}

		// The `createsuperuser` management command resolves the registered
		// `SuperuserCreator` from the framework's inventory at dispatch
		// time. Since reinhardt-web#4522, any `#[user] + #[model]` struct
		// (including the tutorial's minimal `User`) auto-registers via
		// `inventory::submit!`, so no manual `register_superuser_creator`
		// call is required here.

		if let Err(e) = execute_from_command_line().await {
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
