//! Reinhardt Project Management CLI for examples-rough2spec
//!
//! Equivalent to Django's manage.py

use reinhardt::commands::execute_from_command_line;
use reinhardt::core::tokio;
use std::process;

use examples_rough2spec as _;

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var(
            "REINHARDT_SETTINGS_MODULE",
            "examples_rough2spec.config.settings",
        );
    }

    if let Err(e) = execute_from_command_line().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
