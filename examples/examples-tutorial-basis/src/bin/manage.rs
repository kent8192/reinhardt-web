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
	use examples_tutorial_basis::apps::users::models::User;
	use reinhardt::commands::execute_from_command_line;
	use reinhardt::reinhardt_auth::{register_superuser_creator, superuser_creator_for};
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

		// Wire up the `createsuperuser` management command for the tutorial's
		// minimal user model. The `#[user(... manager = false)]` macro path
		// does not (currently) auto-register a `SuperuserCreator` for models
		// without `full = true` — tracked upstream as reinhardt-web#4522.
		// Must run before `execute_from_command_line()` because the framework
		// resolves the registered creator at command-dispatch time.
		register_superuser_creator(superuser_creator_for::<User>());

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
