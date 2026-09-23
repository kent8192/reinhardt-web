//! Reinhardt Project Management CLI for {{ project_name }}
//!
//! This is the project-specific management command interface (equivalent to Django's manage.py).
//!
//! This binary is intentionally native-only. The whole module body is gated
//! behind `not(target_arch = "wasm32")` so that
//! `cargo check --target wasm32-unknown-unknown` on the workspace does not
//! try to compile a tokio-based CLI for the browser target. The wasm side
//! still requires a `main` symbol for `bin` crate-types, so we keep an
//! empty stub.
//!
//! ## Router Registration
//!
//! URL patterns are automatically registered by the framework.
//! No manual registration is required - see `src/config/urls.rs` for the
//! `#[routes]` attribute macro that enables this.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    // Force-link the parent library so its `#[routes]` / `#[model]`
    // `inventory::submit!` registrations survive dead-code elimination.
    // Referencing `get_settings` alone does not guarantee the whole crate
    // (and thus every inventory entry) is linked.
    use {{ crate_name }} as _;
    #[cfg(feature = "commands-shell")]
    use {{ crate_name }}::config::shell::get_shell_config;
    use {{ crate_name }}::config::settings::{get_scoped_settings, get_settings, ProjectSettings};
    use reinhardt::commands::{
        command_error_exit_code, CapabilityProvider, CargoCheckContext, CommandRegistry,
    };
    #[cfg(not(feature = "commands-shell"))]
    use reinhardt::commands::execute_from_command_line_with_capabilities;
    #[cfg(feature = "commands-shell")]
    use reinhardt::commands::execute_from_command_line_with_capabilities_and_shell;
    use reinhardt::conf::settings::builder::BuildError;
    use reinhardt::conf::settings::scoped::ScopedSettings;
    use reinhardt::conf::settings::PendingSettings;
    use std::path::PathBuf;
    use std::process;

    struct ProjectProvider;

    impl CapabilityProvider for ProjectProvider {
        type Settings = ProjectSettings;

        fn scoped_settings(&self) -> Result<ScopedSettings, BuildError> {
            get_scoped_settings()
        }

        fn full_settings(&self) -> Result<PendingSettings<ProjectSettings>, BuildError> {
            get_settings()
        }
    }

    #[tokio::main]
    pub(super) async fn main() {
        // Set settings module environment variable
        // SAFETY: Called at program start before any spawned tasks.
        unsafe {
            std::env::set_var("REINHARDT_SETTINGS_MODULE", "{{ project_name }}.config.settings");
        }

        // The command is selected before either settings provider runs.
        // Static commands resolve selected asset inputs; runtime commands
        // retain the full composed-settings validation path.
        let cargo_context = CargoCheckContext::from_launcher(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            Some(env!("CARGO_PKG_NAME").to_owned()),
            Some("manage".to_owned()),
        );
        #[cfg(feature = "commands-shell")]
        let result =
            execute_from_command_line_with_capabilities_and_shell(
                CommandRegistry::new(),
                ProjectProvider,
                Some(cargo_context),
                get_shell_config(),
            )
                .await;
        #[cfg(not(feature = "commands-shell"))]
        let result = execute_from_command_line_with_capabilities(
            CommandRegistry::new(), ProjectProvider, Some(cargo_context),
        ).await;

        if let Err(e) = result {
            #[cfg(feature = "commands-shell")]
            let exit_code = command_error_exit_code(&e);
            #[cfg(not(feature = "commands-shell"))]
            let exit_code = command_error_exit_code(e.as_ref());
            eprintln!("Error: {}", e);
            process::exit(exit_code);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    reinhardt::commands::shell_runtime_hook();
    native::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
