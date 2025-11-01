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
//! - **AST-Based Code Generation**: Robust code generation using Abstract Syntax Trees
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
//!
//! ## AST-Based Code Generation
//!
//! The `startapp` command uses Abstract Syntax Tree (AST) parsing via `syn` and `quote`
//! for robust code generation and modification. This approach provides several benefits:
//!
//! ### Benefits of AST Approach
//!
//! 1. **Syntax Awareness**: Understands code structure, not just text patterns
//!    - Correctly distinguishes `pub mod app;` from `// pub mod app;` (commented)
//!    - Handles variations in whitespace and formatting automatically
//!
//! 2. **Duplicate Detection**: Structurally detects existing declarations
//!    - Avoids adding duplicate module declarations
//!    - Works correctly even with complex existing code
//!
//! 3. **Consistent Formatting**: Uses `prettyplease` for standardized output
//!    - Ensures consistent code style across generated files
//!    - Integrates well with rustfmt
//!
//! ### Example: apps.rs Generation
//!
//! When you run `startapp myapp`, the command:
//! 1. Parses existing `src/apps.rs` using `syn::parse_file`
//! 2. Checks for existing `pub mod myapp;` declaration structurally
//! 3. Adds new module and use declarations if not present
//! 4. Formats output with `prettyplease::unparse`
//!
//! ```rust,ignore
//! // Generated apps.rs
//! pub mod myapp;
//! pub use myapp::MyappConfig;
//! ```
//!
//! This is more reliable than string-based approaches that can be confused by
//! comments, unusual formatting, or complex code patterns.

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
pub use template::{TemplateCommand, TemplateContext, generate_secret_key, to_camel_case};

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
