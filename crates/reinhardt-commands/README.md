# reinhardt-commands

Django-style management command framework for Reinhardt.

## Overview

`reinhardt-commands` provides a Django-inspired command-line interface for managing Reinhardt projects. It includes built-in commands for database migrations, static file collection, development server, and more.

## Installation

### As a library (for use in `manage.rs`)

Add to your project's `Cargo.toml`:

```toml
[dependencies]
reinhardt-commands = "0.1.0"
```

### As a global CLI tool

For creating new projects and apps, use the separate `reinhardt-admin` package:

```bash
cargo install reinhardt-admin
```

This installs the `reinhardt-admin` command:

```bash
reinhardt-admin startproject myproject
reinhardt-admin startapp myapp
```

See [reinhardt-admin documentation](../reinhardt-admin/README.md) for more details.

## Features

### Built-in Commands

- **makemigrations** - Create new database migrations based on model changes
- **migrate** - Apply database migrations
- **runserver** - Start the development server
- **shell** - Run an interactive REPL
- **check** - Check the project for common issues
- **collectstatic** - Collect static files into `STATIC_ROOT`
- **showurls** - Display all registered URL patterns (requires `routers` feature)

### Feature Flags

- `migrations` - Enable migration-related commands (requires `reinhardt-migrations`)
- `routers` - Enable URL-related commands (requires `reinhardt-routers`)

## Usage

### In Project (`manage.rs`)

Create a `manage.rs` in your project's `src/bin/` directory:

```rust
use clap::{Parser, Subcommand};
use reinhardt_commands::{
    CheckCommand, CommandContext, MakeMigrationsCommand,
    MigrateCommand, RunServerCommand,
};

#[derive(Parser)]
#[command(name = "manage")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Makemigrations {
        #[arg(value_name = "APP_LABEL")]
        app_labels: Vec<String>,

        #[arg(long)]
        dry_run: bool,
    },
    Migrate {
        #[arg(value_name = "APP_LABEL")]
        app_label: Option<String>,
    },
    // ... other commands
}

#[tokio::main]
async fn main() {
    // Parse CLI and execute commands
    // See templates/project/src/bin/manage.rs for complete example
}
```

Then run commands with:

```bash
cargo run --bin manage makemigrations
cargo run --bin manage migrate
cargo run --bin manage runserver
```

### Django Equivalents

| Django                            | Reinhardt                               |
|-----------------------------------|-----------------------------------------|
| `python manage.py makemigrations` | `cargo run --bin manage makemigrations` |
| `python manage.py migrate`        | `cargo run --bin manage migrate`        |
| `python manage.py runserver`      | `cargo run --bin manage runserver`      |
| `python manage.py shell`          | `cargo run --bin manage shell`          |
| `python manage.py check`          | `cargo run --bin manage check`          |
| `python manage.py collectstatic`  | `cargo run --bin manage collectstatic`  |
| `django-admin startproject`       | `reinhardt-admin startproject`          |
| `django-admin startapp`           | `reinhardt-admin startapp`              |

## Custom Commands

Create custom commands by implementing the `BaseCommand` trait:

```rust
use reinhardt_commands::{BaseCommand, CommandContext, CommandResult};
use async_trait::async_trait;

struct MyCommand;

#[async_trait]
impl BaseCommand for MyCommand {
    fn name(&self) -> &str {
        "mycommand"
    }

    fn description(&self) -> &str {
        "My custom command"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        ctx.info("Executing my command!");
        Ok(())
    }
}
```

Register your command in `manage.rs`:

```rust
use reinhardt_commands::CommandRegistry;

let mut registry = CommandRegistry::new();
registry.register(Box::new(MyCommand));
```

## Project Templates

`reinhardt-commands` includes project and app templates:

### Project Templates

- **MTV** (Model-Template-View) - Traditional server-rendered web applications
- **RESTful** - API-first applications

```bash
reinhardt-admin startproject myproject --template-type restful
```

### App Templates

```bash
reinhardt-admin startapp myapp --template-type restful
```

Templates are embedded in the binary using `rust-embed` for fast, dependency-free project generation.

## Architecture

`reinhardt-commands` is designed to be:

- **Independent** - Can be installed and used standalone
- **Composable** - Commands can be combined and extended
- **Feature-gated** - Optional dependencies reduce compile times
- **Django-compatible** - Familiar interface for Django developers

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.