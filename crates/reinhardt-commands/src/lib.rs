//! # Reinhardt Management Commands
//!
//! Django-style management command framework for Reinhardt.
//!
//! ## Features
//!
//! - **BaseCommand**: Trait for creating custom commands
//! - **Standard Commands**: migrate, shell, runserver, etc.
//! - **Argument Parsing**: Clap-based argument handling
//! - **Command Registry**: Automatic command discovery
//! - **Interactive Mode**: Support for interactive prompts
//! - **Colored Output**: Rich terminal output
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_commands::{BaseCommand, CommandContext, CommandResult};
//!
//! struct MyCommand;
//!
//! #[async_trait]
//! impl BaseCommand for MyCommand {
//!     fn name(&self) -> &str {
//!         "mycommand"
//!     }
//!
//!     async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
//!         println!("Hello from my command!");
//!         Ok(())
//!     }
//! }
//! ```

pub mod base;
pub mod builtin;
pub mod collectstatic;
pub mod context;
pub mod embedded_templates;
pub mod formatter;
pub mod i18n_commands;
pub mod mail_commands;
pub mod output;
pub mod registry;
pub mod start_commands;
pub mod template;

use thiserror::Error;

pub use base::{BaseCommand, CommandArgument, CommandOption};
pub use builtin::{MakeMigrationsCommand, MigrateCommand, RunServerCommand, ShellCommand};
pub use collectstatic::{CollectStaticCommand, CollectStaticOptions, CollectStaticStats};
pub use context::CommandContext;
pub use i18n_commands::{CompileMessagesCommand, MakeMessagesCommand};
pub use mail_commands::SendTestEmailCommand;
pub use output::OutputWrapper;
pub use registry::CommandRegistry;
pub use start_commands::{StartAppCommand, StartProjectCommand};
pub use template::{generate_secret_key, to_camel_case, TemplateCommand, TemplateContext};

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type CommandResult<T> = std::result::Result<T, CommandError>;
